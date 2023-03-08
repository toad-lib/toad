use core::fmt::Write;

use toad_common::{Array, GetSize, InsertError, Map, ResultExt, Stem};
use toad_msg::{Token, Type};

use super::{Step, StepOutput};
use crate::net::Addrd;
use crate::platform::{Effect, PlatformTypes};
use crate::req::Req;
use crate::resp::Resp;
use crate::todo::String;
use crate::{exec_inner_step, platform};

/// Struct responsible for buffering and yielding responses to the request
/// we're polling for.
///
/// For more information, see the [module documentation](crate::step::buffer_responses).
#[derive(Debug)]
pub struct HandleAcks<S, B> {
  buffer: Stem<B>,
  inner: S,
}

impl<S, B> HandleAcks<S, B> {
  fn warn_ack_ignored<P: PlatformTypes>(msg: Addrd<&platform::Message<P>>) -> String<1000> {
    let mut string = String::<1000>::default();
    write!(string,
           "IGNORING {}b ACK from {} {:?}",
           msg.data().get_size(),
           msg.addr(),
           msg.data().token).ok();
    string
  }

  fn info_acked<P: PlatformTypes>(msg: Addrd<&platform::Message<P>>) -> String<1000> {
    let mut string = String::<1000>::default();
    write!(string,
           "Got {}b ACK from {} for {:?}",
           msg.data().get_size(),
           msg.addr(),
           (msg.data().id, msg.data().token)).ok();
    string
  }
}

impl<S: Default, B: Default> Default for HandleAcks<S, B> {
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
  /// Only applicable to [`HandleAcks`] that uses `ArrayVec` or
  /// similar heapless backing structure.
  ConBufferCapacityExhausted,
}

impl<E> From<E> for Error<E> {
  fn from(e: E) -> Self {
    Error::Inner(e)
  }
}

impl<E: core::fmt::Debug> core::fmt::Debug for Error<E> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    match self {
      | Self::ConBufferCapacityExhausted => f.debug_struct("ConBufferCapacityExhausted").finish(),
      | Self::Inner(e) => e.fmt(f),
    }
  }
}

impl<E: super::Error> super::Error for Error<E> {}

macro_rules! common {
  ($in:expr, $msg:expr, $effects:expr, $buffer:expr) => {{
    let msg: Addrd<&platform::Message<P>> = $msg;

    if msg.data().ty == Type::Ack && !$buffer.map_ref(|buf| buf.has(&msg.map(|m| m.token))) {
      $effects.push(Effect::Log(log::Level::Warn, Self::warn_ack_ignored::<P>(msg)));
      None
    } else if msg.data().ty == Type::Ack {
      $effects.push(Effect::Log(log::Level::Trace, Self::info_acked::<P>(msg)));
      $buffer.map_mut(|buf| buf.remove(&msg.as_ref().map(|m| m.token)));
      None
    } else {
      Some(Ok($in))
    }
  }};
}

impl<P: PlatformTypes,
      B: Map<Addrd<Token>, ()> + core::fmt::Debug,
      E: super::Error,
      S: Step<P, PollReq = Addrd<Req<P>>, PollResp = Addrd<Resp<P>>, Error = E>> Step<P>
  for HandleAcks<S, B>
{
  type PollReq = Addrd<Req<P>>;
  type PollResp = Addrd<Resp<P>>;
  type Error = Error<E>;
  type Inner = S;

  fn inner(&self) -> &S {
    &self.inner
  }

  fn poll_req(&self,
              snap: &crate::platform::Snapshot<P>,
              effects: &mut <P as PlatformTypes>::Effects)
              -> StepOutput<Self::PollReq, Self::Error> {
    let req = exec_inner_step!(self.inner.poll_req(snap, effects), Error::Inner);

    match req {
      | Some(req) => {
        let msg = req.as_ref().map(|r| r.as_ref());
        common!(req, msg, effects, self.buffer)
      },
      | None => None,
    }
  }

  fn poll_resp(&self,
               snap: &crate::platform::Snapshot<P>,
               effects: &mut <P as PlatformTypes>::Effects,
               token: toad_msg::Token,
               addr: no_std_net::SocketAddr)
               -> StepOutput<Self::PollResp, Self::Error> {
    let resp = exec_inner_step!(self.inner.poll_resp(snap, effects, token, addr),
                                Error::Inner);

    match resp {
      | Some(resp) => {
        let msg = resp.as_ref().map(|r| r.as_ref());
        common!(resp, msg, effects, self.buffer)
      },
      | None => None,
    }
  }

  fn on_message_sent(&self,
                     snap: &platform::Snapshot<P>,
                     msg: &Addrd<crate::platform::Message<P>>)
                     -> Result<(), Self::Error> {
    self.inner
        .on_message_sent(snap, msg)
        .map_err(Error::Inner)?;

    match msg.data().ty {
      | Type::Con => self.buffer
                         .map_mut(|buf| buf.insert(msg.as_ref().map(|m| m.token), ()))
                         .recover(|e| {
                           if matches!(e, InsertError::Exists(_)) {
                             Ok(())
                           } else {
                             Err(e)
                           }
                         })
                         .map_err(|_| Error::ConBufferCapacityExhausted),
      | _ => Ok(()),
    }
  }
}

#[cfg(test)]
mod test {
  use std::collections::BTreeMap;

  use tinyvec::array_vec;

  use super::*;
  use crate::platform::Effect;
  use crate::step::test::test_step;

  type InnerPollReq = Addrd<Req<crate::test::Platform>>;
  type InnerPollResp = Addrd<Resp<crate::test::Platform>>;
  type HandleAcks<S> = super::HandleAcks<S, BTreeMap<Addrd<Token>, ()>>;

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
    GIVEN HandleAcks::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
    WHEN inner_errors [
      (inner.poll_req => { Some(Err(nb::Error::Other(()))) }),
      (inner.poll_resp => { Some(Err(nb::Error::Other(()))) }),
      (inner.on_message_sent = { |_, _| Err(()) })
    ]
    THEN this_should_error [
      (poll_req(_, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::Other(Error::Inner(()))))) }),
      (poll_resp(_, _, _, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::Other(Error::Inner(()))))) }),
      (on_message_sent(_, test_message(Type::Con)) should satisfy { |out| assert_eq!(out, Err(Error::Inner(()))) })
    ]
  );

  test_step!(
    GIVEN HandleAcks::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
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
    GIVEN HandleAcks::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
    WHEN unexpected_ack_received [
      (inner.poll_req => { Some(Ok(test_message(Type::Ack).map(Req::from))) }),
      (inner.poll_resp => { Some(Ok(test_message(Type::Ack).map(Resp::from))) }),
      (inner.on_message_sent = { |_, _| Ok(()) })
    ]
    THEN should_ignore [
      (
        poll_resp(
          _,
          _,
          test_message(Type::Con).data().token,
          crate::test::dummy_addr()
        ) should satisfy {
          |out| assert_eq!(out, None)
        }
      ),
      (poll_req(_, _) should satisfy { |out| assert_eq!(out, None) }),
      (
        effects should satisfy {|effects| {
          assert!(matches!(effects[0], Effect::Log(log::Level::Warn, _)));
          assert!(matches!(effects[1], Effect::Log(log::Level::Warn, _)));
        }}
      )
    ]
  );

  test_step!(
    GIVEN HandleAcks::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
    WHEN expected_ack_received [
      (inner.poll_req => { Some(Ok(test_message(Type::Ack).map(Req::from))) }),
      (inner.poll_resp => { Some(Ok(test_message(Type::Ack).map(Resp::from))) }),
      (inner.on_message_sent = { |_, _| Ok(()) })
    ]
    THEN all_good [
      (on_message_sent(_, test_message(Type::Con)) should satisfy { |_| () }),
      (
        on_message_sent(
          _,
          {
            let Addrd(mut msg, addr) = test_message(Type::Con);
            msg.token = Token(array_vec!(_ => 2));
            Addrd(msg, addr)
          }
        ) should satisfy { |_| () }
      ),
      (
        poll_resp(
          _,
          _,
          test_message(Type::Con).data().token,
          crate::test::dummy_addr()
        ) should satisfy {
          |out| assert!(out.is_none())
        }
      ),
      (effects should satisfy {|eff| assert!(matches!(eff.as_slice(), &[Effect::Log(_, _)]))})
    ]
  );
}
