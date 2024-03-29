use core::fmt::Write;

use no_std_net::SocketAddr;
use toad_array::Array;
use toad_len::Len;
use toad_map::Map;
use toad_msg::{Token, Type};
use toad_stem::Stem;

use super::{Step, StepOutput};
use crate::exec_inner_step;
use crate::net::Addrd;
use crate::platform::{Effect, PlatformTypes};
use crate::req::Req;
use crate::resp::Resp;
use crate::todo::String;

/// Struct responsible for buffering and yielding responses to the request
/// we're polling for.
///
/// For more information, see the [module documentation](crate::step::buffer_responses).
#[derive(Debug)]
pub struct BufferResponses<S, B> {
  buffer: Stem<B>,
  inner: S,
}

impl<S: Default, B: Default> Default for BufferResponses<S, B> {
  fn default() -> Self {
    Self { buffer: Default::default(),
           inner: S::default() }
  }
}

impl<S, B> BufferResponses<S, B> {
  fn store<P>(&self, resp: Addrd<Resp<P>>)
    where P: PlatformTypes,
          B: Map<(SocketAddr, Token, Type), Addrd<Resp<P>>>
  {
    let mut resp_removable = Some(resp);
    self.buffer.map_mut(|buf| {
                 let resp = Option::take(&mut resp_removable).unwrap();
                 buf.insert((resp.addr(), resp.data().as_ref().token, resp.data().as_ref().ty),
                            resp)
                    .ok()
               });
  }
}

/// Errors that can be encountered when buffering responses
#[derive(Clone, PartialEq, Eq)]
pub enum Error<E> {
  /// The inner step failed.
  ///
  /// This variant's Debug representation is completely
  /// replaced by the inner type E's debug representation
  Inner(E),
  /// Storing this response would exceed a hard capacity for the
  /// response buffer.
  ///
  /// Only applicable to [`BufferResponses`] that uses `ArrayVec` or
  /// similar heapless backing structure.
  BufferResponsesFull,
}

impl<E> From<E> for Error<E> {
  fn from(e: E) -> Self {
    Error::Inner(e)
  }
}

impl<E: core::fmt::Debug> core::fmt::Debug for Error<E> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    match self {
      | Self::BufferResponsesFull => f.debug_struct("BufferResponsesFull").finish(),
      | Self::Inner(e) => e.fmt(f),
    }
  }
}

impl<E: super::Error> super::Error for Error<E> {}

impl<P: PlatformTypes,
      B: Map<(SocketAddr, Token, Type), Addrd<Resp<P>>>,
      E: super::Error,
      S: Step<P, PollReq = Addrd<Req<P>>, PollResp = Addrd<Resp<P>>, Error = E>> Step<P>
  for BufferResponses<S, B>
{
  type PollReq = Addrd<Req<P>>;
  type PollResp = Addrd<Resp<P>>;
  type Error = Error<E>;
  type Inner = S;

  fn inner(&self) -> &Self::Inner {
    &self.inner
  }

  fn poll_req(&self,
              snap: &crate::platform::Snapshot<P>,
              effects: &mut <P as PlatformTypes>::Effects)
              -> StepOutput<Self::PollReq, Self::Error> {
    self.inner
        .poll_req(snap, effects)
        .map(|o| o.map_err(|e| e.map(Error::Inner)))
  }

  fn poll_resp(&self,
               snap: &crate::platform::Snapshot<P>,
               effects: &mut <P as PlatformTypes>::Effects,
               token: toad_msg::Token,
               addr: no_std_net::SocketAddr)
               -> StepOutput<Self::PollResp, Self::Error> {
    let resp = exec_inner_step!(self.inner.poll_resp(snap, effects, token, addr),
                                Error::Inner);

    if self.buffer.map_ref(Len::is_full) {
      return Some(Err(nb::Error::Other(Error::BufferResponsesFull)));
    }

    let try_remove_from_buffer =
      |ty: Type| self.buffer.map_mut(|buf| buf.remove(&(addr, token, ty)));

    let is_what_we_polled_for =
      |resp: &Addrd<Resp<_>>| resp.addr() == addr && resp.data().as_ref().token == token;

    match resp {
      | Some(resp) if is_what_we_polled_for(&resp) => Some(Ok(resp)),
      | Some(resp) => {
        let mut msg = String::<1000>::default();
        write!(&mut msg,
               "polled for response to {:?}, got response with token {:?}",
               token,
               resp.data().token()).ok();
        effects.push(Effect::Log(log::Level::Info, msg));
        self.store(resp);

        match try_remove_from_buffer(Type::Ack).or_else(|| try_remove_from_buffer(Type::Con))
                                               .or_else(|| try_remove_from_buffer(Type::Non))
                                               .or_else(|| try_remove_from_buffer(Type::Reset))
        {
          | Some(resp) => Some(Ok(resp)),
          | None => Some(Err(nb::Error::WouldBlock)),
        }
      },
      | None => None,
    }
  }
}

#[cfg(test)]
mod test {
  use std::collections::BTreeMap;

  use tinyvec::array_vec;
  use toad_msg::Id;

  use super::*;
  use crate::step::test::test_step;
  use crate::test::Platform as P;

  type InnerPollReq = Addrd<Req<P>>;
  type InnerPollResp = Addrd<Resp<P>>;
  type BufferResponses<S> =
    super::BufferResponses<S, BTreeMap<(SocketAddr, Token, Type), Addrd<Resp<P>>>>;

  test_step!(
    GIVEN BufferResponses::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
    WHEN inner_errors [
      (inner.poll_req => { Some(Err(nb::Error::Other(()))) }),
      (inner.poll_resp => { Some(Err(nb::Error::Other(()))) })
    ]
    THEN this_should_error [
      (poll_req(_, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::Other(Error::Inner(()))))) }),
      (poll_resp(_, _, _, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::Other(Error::Inner(()))))) })
    ]
  );

  test_step!(
    GIVEN BufferResponses::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
    WHEN inner_blocks [
      (inner.poll_req => { Some(Err(nb::Error::WouldBlock)) }),
      (inner.poll_resp => { Some(Err(nb::Error::WouldBlock)) })
    ]
    THEN this_should_block [
      (poll_req(_, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::WouldBlock))) }),
      (poll_resp(_, _, _, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::WouldBlock))) })
    ]
  );

  test_step!(
    GIVEN BufferResponses::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
    WHEN inner_yields_request [
      (inner.poll_req => {{
        use toad_msg::*;

        let msg = platform::Message::<P> {
          ver: Default::default(),
          token: Token(Default::default()),
          ty: Type::Con,
          code: Code::new(1, 01),
          id: Id(1),
          opts: Default::default(),
          payload: Payload(vec![]),
        };

        Some(Ok(Addrd(msg.into(), crate::test::dummy_addr())))
      }})
    ]
    THEN this_should_pass_through [
      (poll_req(_, _) should satisfy { |out| assert_eq!(out.unwrap().unwrap().data().as_ref().id, Id(1)) })
    ]
  );

  test_step!(
    GIVEN BufferResponses::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
    WHEN inner_yields_response [
      (inner.poll_resp = {
        |_, _, _, _| {
          use toad_msg::*;
          use no_std_net::SocketAddr;

          static mut CALL: u8 = 1;

          struct Case {
            ty: Type,
            token: u8,
            id: Id,
            addr: SocketAddr,
          }

          let addr_1 = crate::test::dummy_addr();
          let addr_2 = crate::test::dummy_addr_2();

          let skip = Case { ty: Type::Reset, token: 255, id: Id(255), addr: addr_2 };

          let Case {token, ty, id, addr} =
            match CALL {
              | 1 => Case { ty: Type::Ack, token: 1, id: Id(1), addr: addr_1 },
              | 2 => Case { ty: Type::Ack, token: 2, id: Id(2), addr: addr_1 },
              | 3 => Case { ty: Type::Ack, token: 1, id: Id(1), addr: addr_2 },
              | 4 => Case { ty: Type::Ack, token: 2, id: Id(1), addr: addr_2 },
              | 5 | 6 => skip,
              | 7 => Case { ty: Type::Ack, token: 3, id: Id(2), addr: addr_2 },
              | 8 => Case { ty: Type::Non, token: 3, id: Id(3), addr: addr_2 },
              | _ => skip,
            };

          CALL += 1;

          let msg = platform::Message::<P> {
            ver: Default::default(),
            token: Token(Some(token).into_iter().collect()),
            ty,
            code: Code::new(1, 01),
            id,
            opts: Default::default(),
            payload: Payload(vec![]),
          };

          Some(Ok(Addrd(msg.into(), addr)))
        }
      })
    ]
    THEN this_should_buffer_and_yield_correct_response [
      (
        poll_resp(
          _,
          _,
          Token(array_vec!([u8; 8] => 2)),
          crate::test::dummy_addr_2()
        ) should satisfy {
          // CACHED: ACK Token(1) Id(1) dummy_addr
          |out| assert_eq!(out, Some(Err(nb::Error::WouldBlock)))
        }
      ),
      (
        poll_resp(
          _,
          _,
          Token(array_vec!([u8; 8] => 2)),
          crate::test::dummy_addr_2()
        ) should satisfy {
          // CACHED: ACK Token(2) Id(2) dummy_addr
          |out| assert_eq!(out, Some(Err(nb::Error::WouldBlock)))
        }
      ),
      (
        poll_resp(
          _,
          _,
          Token(array_vec!([u8; 8] => 2)),
          crate::test::dummy_addr_2()
        ) should satisfy {
          // CACHED: ACK Token(1) Id(1) dummy_addr_2
          |out| assert_eq!(out, Some(Err(nb::Error::WouldBlock)))
        }
      ),
      (
        poll_resp(
          _,
          _,
          Token(array_vec!([u8; 8] => 2)),
          crate::test::dummy_addr_2()
        ) should satisfy {
          // POPPED: ACK Token(2) Id(2) dummy_addr_2
          |out| assert_eq!(out.expect("a").expect("a").data().as_ref().id, Id(1))
        }
      ),
      (
        poll_resp(
          _,
          _,
          toad_msg::Token(array_vec!([u8; 8] => 1)),
          crate::test::dummy_addr()
        ) should satisfy {
          // POPPED: ACK Token(1) Id(1) dummy_addr
          |out| assert_eq!(out.expect("b").expect("b").data().as_ref().id, Id(1))
        }
      ),
      (
        poll_resp(
          _,
          _,
          Token(array_vec!([u8; 8] => 2)),
          crate::test::dummy_addr()
        ) should satisfy {
          // POPPED: ACK Token(2) Id(2) dummy_addr
          |out| assert_eq!(out.expect("c").expect("c").data().as_ref().id, Id(2))
        }
      ),
      (poll_resp(_, _, _, _) should satisfy { |_| () } ), // CACHED: ACK Token(3) Id(2) dummy_addr_2
      (poll_resp(_, _, _, _) should satisfy { |_| () } ), // CACHED: NON Token(3) Id(3) dummy_addr_2
      (
        poll_resp(
         _,
          _,
          Token(array_vec!([u8; 8] => 3)),
          crate::test::dummy_addr_2()
        ) should satisfy {
          |out| {
            // POPPED: ACK Token(1) Id(2) dummy_addr_2
            let msg = out.expect("d").expect("d").unwrap();
            assert_eq!(msg.as_ref().id, Id(2));
            assert_eq!(msg.as_ref().ty, Type::Ack);
          }
        }
      ),
      (
        poll_resp(
          _,
          _,
          Token(array_vec!([u8; 8] => 3)),
          crate::test::dummy_addr_2()
        ) should satisfy {
          |out| {
            // POPPED: NON Token(1) Id(3) dummy_addr_2
            let msg = out.expect("e").expect("e").unwrap();
            assert_eq!(msg.as_ref().id, Id(3));
            assert_eq!(msg.as_ref().ty, Type::Non);
          }
        }
      )
    ]
  );
}
