use tinyvec::ArrayVec;
use toad_common::Map;
use toad_msg::{Id, Token};

use super::{Step, StepOutput};
use crate::exec_inner_step;
use crate::net::Addrd;
use crate::platform::Platform;
use crate::req::Req;
use crate::resp::Resp;

/// `BufferResponses` that uses BTreeMap
///
/// Only enabled when feature "alloc" enabled.
#[cfg(feature = "alloc")]
pub mod alloc {
  use ::std_alloc::collections::BTreeMap;

  use super::*;

  /// `BufferResponses` that uses BTreeMap
  ///
  /// Only enabled when feature "alloc" enabled.
  ///
  /// For more information see [`super::BufferResponses`]
  /// or the [module documentation](crate::step::buffer_responses).
  pub type BufferResponses<S, P> =
    super::BufferResponses<S, BTreeMap<Addrd<Token>, Addrd<Resp<P>>>>;
}

/// `BufferResponses` that does not use
/// heap allocation and stores the buffer on the stack.
pub mod no_alloc {
  use super::*;

  /// `BufferResponses` that does not use
  /// heap allocation and stores the buffer on the stack.
  ///
  /// For more information see [`super::BufferResponses`]
  /// or the [module documentation](crate::step::buffer_responses).
  pub type BufferResponses<S, P> =
    super::BufferResponses<S, ArrayVec<[(Addrd<Token>, Addrd<Resp<P>>); 16]>>;
}

/// Struct responsible for buffering and yielding responses to the request
/// we're polling for.
///
/// For more information, see the [module documentation](crate::step::buffer_responses).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BufferResponses<S, B> {
  buffer: B,
  inner: S,
}

impl<S: Default, B: Default> Default for BufferResponses<S, B> {
  fn default() -> Self {
    Self { buffer: Default::default(),
           inner: S::default() }
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
  CapacityExhausted,
}

impl<E: core::fmt::Debug> core::fmt::Debug for Error<E> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    match self {
      | Self::CapacityExhausted => f.debug_struct("CapacityExhausted").finish(),
      | Self::Inner(e) => e.fmt(f),
    }
  }
}

impl<E: super::Error> super::Error for Error<E> {}

impl<P: Platform,
      B: Map<Addrd<Token>, Addrd<Resp<P>>>,
      E: super::Error,
      S: Step<P, PollReq = Addrd<Req<P>>, PollResp = Addrd<Resp<P>>, Error = E>> Step<P>
  for BufferResponses<S, B>
{
  type PollReq = Addrd<Req<P>>;
  type PollResp = Addrd<Resp<P>>;
  type Error = Error<E>;

  fn poll_req(&mut self,
              snap: &crate::platform::Snapshot<P>,
              effects: &mut <P as Platform>::Effects)
              -> StepOutput<Self::PollReq, Self::Error> {
    self.inner
        .poll_req(snap, effects)
        .map(|o| o.map_err(|e| e.map(Error::Inner)))
  }

  fn poll_resp(&mut self,
               snap: &crate::platform::Snapshot<P>,
               effects: &mut <P as Platform>::Effects,
               token: toad_msg::Token,
               addr: no_std_net::SocketAddr)
               -> StepOutput<Self::PollResp, Self::Error> {
    let resp = exec_inner_step!(self.inner.poll_resp(snap, effects, token, addr),
                                Error::Inner);

    match resp {
      | Some(resp) if Addrd(token, addr) == resp.as_ref().map(|r| r.msg.token) => Some(Ok(resp)),
      | Some(_) if self.buffer.is_full() => Some(Err(nb::Error::Other(Error::CapacityExhausted))),
      | Some(resp) => {
        self.buffer
            .insert(resp.as_ref().map(|r| r.msg.token), resp)
            .ok();

        match self.buffer.remove(&Addrd(token, addr)) {
          | Some(resp) => Some(Ok(resp)),
          | None => Some(Err(nb::Error::WouldBlock)),
        }
      },
      | None => None,
    }
  }

  fn message_sent(&mut self, msg: &Addrd<crate::platform::Message<P>>) -> Result<(), Self::Error> {
    self.inner.message_sent(msg).map_err(Error::Inner)
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::step::test::test_step;

  type InnerPollReq = Addrd<Req<crate::test::Platform>>;
  type InnerPollResp = Addrd<Resp<crate::test::Platform>>;

  test_step!(
    GIVEN alloc::BufferResponses::<Dummy, crate::test::Platform> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
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
    GIVEN alloc::BufferResponses::<Dummy, crate::test::Platform> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
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
    GIVEN alloc::BufferResponses::<Dummy, crate::test::Platform> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
    WHEN inner_yields_request [
      (inner.poll_req => {{
        use toad_msg::*;

        let msg = platform::Message::<crate::test::Platform> {
          ver: Default::default(),
          token: Token(Default::default()),
          ty: Type::Con,
          code: Code::new(1, 01),
          id: Id(1),
          opts: vec![],
          payload: Payload(vec![]),
        };

        Some(Ok(Addrd(msg.into(), crate::test::dummy_addr())))
      }})
    ]
    THEN this_should_pass_through [
      (poll_req(_, _) should satisfy { |out| assert_eq!(out.unwrap().unwrap().data().msg.id, Id(1)) })
    ]
  );

  type Out = StepOutput<Addrd<Resp<crate::test::Platform>>, Error<()>>;
  test_step!(
    GIVEN alloc::BufferResponses::<Dummy, crate::test::Platform> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
    WHEN inner_yields_response [
      (inner.poll_resp = {
        || {
          use toad_msg::*;
          use no_std_net::SocketAddr;

          static mut CALL: u8 = 1;

          struct Case {
            token: Token,
            id: Id,
            addr: SocketAddr,
          }

          let Case {token, id, addr} =
            match CALL {
              1 => Case {
                token: Token(ArrayVec::from([1; 8])),
                id: Id(1),
                addr: crate::test::dummy_addr(),
              },
              2 => Case {
                token: Token(ArrayVec::from([2; 8])),
                id: Id(2),
                addr: crate::test::dummy_addr(),
              },
              3 => Case {
                token: Token(ArrayVec::from([1; 8])),
                id: Id(1),
                addr: crate::test::dummy_addr_2(),
              },
              4 => Case {
                token: Token(ArrayVec::from([2; 8])),
                id: Id(2),
                addr: crate::test::dummy_addr_2(),
              },
              _ => Case {
                token: Token(ArrayVec::from([CALL; 8])),
                id: Id(2),
                addr: crate::test::dummy_addr_2(),
              },
            };

          CALL += 1;

          let msg = platform::Message::<crate::test::Platform> {
            ver: Default::default(),
            token,
            ty: Type::Con,
            code: Code::new(1, 01),
            id,
            opts: vec![],
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
          toad_msg::Token(ArrayVec::from([2; 8])),
          crate::test::dummy_addr_2()
        ) should satisfy {
          |out: Out| assert_eq!(out, Some(Err(nb::Error::WouldBlock)))
        }
      ),
      // CACHED: Token([1; 8]) Id(1) dummy_addr
      (
        poll_resp(
          _,
          _,
          toad_msg::Token(ArrayVec::from([2; 8])),
          crate::test::dummy_addr_2()
        ) should satisfy {
          |out: Out| assert_eq!(out, Some(Err(nb::Error::WouldBlock)))
        }
      ),
      // CACHED: Token([2; 8]) Id(2) dummy_addr
      (
        poll_resp(
          _,
          _,
          toad_msg::Token(ArrayVec::from([2; 8])),
          crate::test::dummy_addr_2()
        ) should satisfy {
          |out: Out| assert_eq!(out, Some(Err(nb::Error::WouldBlock)))
        }
      ),
      // CACHED: Token([1; 8]) Id(1) dummy_addr_2
      (
        poll_resp(
          _,
          _,
          toad_msg::Token(ArrayVec::from([2; 8])),
          crate::test::dummy_addr_2()
        ) should satisfy {
          |out: Out| assert_eq!(out.expect("a").expect("a").data().msg.id, Id(2))
        }
      ),
      // RETURNED: Token([2; 8]) Id(2) dummy_addr_2
      (
        poll_resp(
          _,
          _,
          toad_msg::Token(ArrayVec::from([1; 8])),
          crate::test::dummy_addr()
        ) should satisfy {
          |out: Out| assert_eq!(out.expect("b").expect("b").data().msg.id, Id(1))
        }
      ),
      // POPPED: Token([1; 8]) Id(1) dummy_addr
      (
        poll_resp(
          _,
          _,
          toad_msg::Token(ArrayVec::from([2; 8])),
          crate::test::dummy_addr()
        ) should satisfy {
          |out: Out| assert_eq!(out.expect("c").expect("c").data().msg.id, Id(2))
        }
      ),
      // POPPED: Token([2; 8]) Id(2) dummy_addr
      (
        poll_resp(
          _,
          _,
          toad_msg::Token(ArrayVec::from([1; 8])),
          crate::test::dummy_addr_2()
        ) should satisfy {
          |out: Out| assert_eq!(out.expect("d").expect("d").data().msg.id, Id(1))
        }
      )
      // POPPED: Token([1; 8]) Id(1) dummy_addr_2
    ]
  );
}
