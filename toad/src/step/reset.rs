use no_std_net::SocketAddr;
use tinyvec::ArrayVec;
use toad_common::{Array, Map};
use toad_msg::to_bytes::MessageToBytesError;
use toad_msg::{Code, Id, Payload, Token, TryIntoBytes, Type};

use super::{Step, StepOutput};
use crate::net::Addrd;
use crate::platform::{Effect, Platform};
use crate::req::Req;
use crate::resp::Resp;
use crate::{exec_inner_step, platform};

/// `BufferResponses` that uses BTreeMap
///
/// Only enabled when feature "alloc" enabled.
#[cfg(feature = "alloc")]
pub mod alloc {
  use ::std_alloc::collections::BTreeMap;

  use super::*;

  /// [`Reset`](super::Reset) that uses BTreeMap
  ///
  /// Only enabled when feature "alloc" enabled.
  ///
  /// For more information see [`super::Reset`]
  /// or the [module documentation](crate::step::reset).
  pub type Reset<S> = super::Reset<S, BTreeMap<Addrd<Token>, ()>>;
}

/// [`Reset`] that does not use
/// heap allocation and stores the buffer on the stack.
pub mod no_alloc {
  use super::*;

  /// [`Reset`](super::Reset) that does not use
  /// heap allocation and stores the buffer on the stack.
  ///
  /// For more information see [`super::Reset`]
  /// or the [module documentation](crate::step::reset).
  pub type Reset<S> = super::Reset<S, ArrayVec<[(Addrd<Token>, ()); 16]>>;
}

/// Struct responsible for buffering and yielding responses to the request
/// we're polling for.
///
/// For more information, see the [module documentation](crate::step::buffer_responses).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Reset<S, B> {
  buffer: B,
  inner: S,
}

impl<S: Default, B: Default> Default for Reset<S, B> {
  fn default() -> Self {
    Self { buffer: Default::default(),
           inner: S::default() }
  }
}

/// Errors that can be encountered when buffering responses
#[derive(Clone, PartialEq)]
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
  /// Failed to serialize outbound Reset message
  SerializingResetFailed(MessageToBytesError),
}

impl<E: core::fmt::Debug> core::fmt::Debug for Error<E> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    match self {
      | Self::SerializingResetFailed(e) => {
        f.debug_tuple("SerializingResetFailed").field(e).finish()
      },
      | Self::CapacityExhausted => f.debug_struct("CapacityExhausted").finish(),
      | Self::Inner(e) => e.fmt(f),
    }
  }
}

impl<E: super::Error> super::Error for Error<E> {}

macro_rules! common {
  ($msg:expr, $effects:expr, $buffer:expr) => {
    let msg = $msg;

    if msg.data().ty == Type::Ack && !$buffer.has(&msg.map(|m| m.token)) {
      let reset = platform::Message::<P> { ver: Default::default(),
                                           ty: Type::Reset,
                                           id: Id(0),
                                           token: msg.data().token,
                                           code: Code::new(0, 0),
                                           payload: Payload(Default::default()),
                                           opts: Default::default() };

      match reset.try_into_bytes() {
        | Ok(dgram) => $effects.push(Effect::SendDgram(Addrd(dgram, msg.addr()))),
        | Err(e) => return Some(Err(nb::Error::Other(Error::SerializingResetFailed(e)))),
      };
    }
  };
}

impl<P: Platform,
      B: Map<Addrd<Token>, ()>,
      E: super::Error,
      S: Step<P, PollReq = Addrd<Req<P>>, PollResp = Addrd<Resp<P>>, Error = E>> Step<P>
  for Reset<S, B>
{
  type PollReq = Addrd<Req<P>>;
  type PollResp = Addrd<Resp<P>>;
  type Error = Error<E>;

  fn poll_req(&mut self,
              snap: &crate::platform::Snapshot<P>,
              effects: &mut <P as Platform>::Effects)
              -> StepOutput<Self::PollReq, Self::Error> {
    let req = exec_inner_step!(self.inner.poll_req(snap, effects), Error::Inner);

    match req {
      | Some(req) => {
        let msg = req.as_ref().map(|r| &r.msg);
        common!(msg, effects, self.buffer);

        Some(Ok(req))
      },
      | None => None,
    }
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
      | Some(resp) => {
        let msg = resp.as_ref().map(|r| &r.msg);
        common!(msg, effects, self.buffer);

        Some(Ok(resp))
      },
      | None => None,
    }
  }

  fn message_sent(&mut self, msg: &Addrd<crate::platform::Message<P>>) -> Result<(), Self::Error> {
    self.inner.message_sent(msg).map_err(Error::Inner)?;

    match msg.data().ty {
      | Type::Con => self.buffer
                         .insert(msg.as_ref().map(|m| m.token), ())
                         .map_err(|_| Error::CapacityExhausted),
      | _ => Ok(()),
    }
  }
}

#[cfg(test)]
mod test {
  use tinyvec::array_vec;

  use super::*;
  use crate::platform::Effect;
  use crate::step::test::test_step;

  type InnerPollReq = Addrd<Req<crate::test::Platform>>;
  type InnerPollResp = Addrd<Resp<crate::test::Platform>>;

  fn test_message(ty: Type) -> Addrd<crate::test::Message> {
    use toad_msg::*;

    Addrd(crate::test::Message { ver: Default::default(),
                                 ty,
                                 id: Id(1),
                                 code: Code::new(1, 1),
                                 token: Token(array_vec!(_ => 1)),
                                 payload: Payload(Default::default()),
                                 opts: Default::default() },
          crate::test::dummy_addr())
  }

  test_step!(
    GIVEN alloc::Reset::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
    WHEN inner_errors [
      (inner.poll_req => { Some(Err(nb::Error::Other(()))) }),
      (inner.poll_resp => { Some(Err(nb::Error::Other(()))) }),
      (inner.message_sent = { |_| Err(()) })
    ]
    THEN this_should_error [
      (poll_req(_, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::Other(Error::Inner(()))))) }),
      (poll_resp(_, _, _, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::Other(Error::Inner(()))))) }),
      (message_sent(test_message(Type::Con)) should satisfy { |out| assert_eq!(out, Err(Error::Inner(()))) })
    ]
  );

  test_step!(
    GIVEN alloc::Reset::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
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
    GIVEN alloc::Reset::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
    WHEN ack_received [
      (inner.poll_req => { Some(Ok(test_message(Type::Ack).map(Req::from))) }),
      (inner.poll_resp => { Some(Ok(test_message(Type::Ack).map(Resp::from))) }),
      (inner.message_sent = { |_| Ok(()) })
    ]
    THEN reset_should_be_sent [
      (
        poll_resp(
          _,
          _,
          test_message(Type::Con).data().token,
          crate::test::dummy_addr()
        ) should satisfy {
          |out| assert!(out.unwrap().is_ok())
        }
      ),
      (
        poll_req(_, _) should satisfy {
          |out| assert!(out.unwrap().is_ok())
        }
      ),
      (
        effects == {{
          use toad_msg::{Id, TryIntoBytes};

          let msg = test_message(Type::Reset);
          let msg = msg.map(|mut msg| {
            msg.id = Id(0);
            msg.code = Code::new(0, 0);
            msg
          });

          vec![
            Effect::SendDgram(msg.clone().map(TryIntoBytes::try_into_bytes).map(Result::unwrap)),
            Effect::SendDgram(msg.map(TryIntoBytes::try_into_bytes).map(Result::unwrap)),
          ]
        }}
      )
    ]
  );

  test_step!(
    GIVEN alloc::Reset::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
    WHEN expected_ack_received [
      (inner.poll_req => { Some(Ok(test_message(Type::Ack).map(Req::from))) }),
      (inner.poll_resp => { Some(Ok(test_message(Type::Ack).map(Resp::from))) }),
      (inner.message_sent = { |_| Ok(()) })
    ]
    THEN all_good [
      (message_sent(test_message(Type::Con)) should satisfy { |_| () }),
      (
        message_sent({
          let Addrd(mut msg, addr) = test_message(Type::Con);
          msg.token = Token(array_vec!(_ => 2));
          Addrd(msg, addr)
        }) should satisfy { |_| () }
      ),
      (
        poll_resp(
          _,
          _,
          test_message(Type::Con).data().token,
          crate::test::dummy_addr()
        ) should satisfy {
          |out| assert!(out.unwrap().is_ok())
        }
      ),
      (effects == {vec![]})
    ]
  );
}
