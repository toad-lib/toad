use core::fmt::Write;

use naan::prelude::ResultExt;
use toad_array::Array;
use toad_len::Len;
use toad_map::{InsertError, Map};
use toad_msg::{Token, Type};
use toad_stem::Stem;

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
  fn warn_ack_ignored<P: PlatformTypes>(msg: Addrd<&platform::Message<P>>,
                                        because: impl AsRef<str>)
                                        -> String<1000> {
    let (size, sender, token, because) =
      (msg.data().len(), msg.addr(), msg.data().token, because.as_ref());

    String::fmt(format_args!("DISCARDING {size}b ACK from {sender} {token:?} because {because}"))
  }

  fn info_acked<P: PlatformTypes>(msg: Addrd<&platform::Message<P>>) -> String<1000> {
    let (size, sender, token) = (msg.data().len(), msg.addr(), (msg.data().id, msg.data().token));
    String::fmt(format_args!("Got {size}b ACK from {sender} for {token:?}",))
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

    match msg.data().ty {
      Type::Ack if msg.data().code.kind() == toad_msg::CodeKind::Empty
          => {
        $effects.push(Effect::Log(log::Level::Warn, Self::warn_ack_ignored::<P>(msg, "the message was empty (0.00)")));
        None
      },
      Type::Ack if !$buffer.map_ref(|buf| buf.has(&msg.map(|m| m.token)))
          => {
        $effects.push(Effect::Log(log::Level::Warn, Self::warn_ack_ignored::<P>(msg, "it was addressing a token that we were not expecting an ACK for")));
        None
      },
      Type::Ack => {
        $effects.push(Effect::Log(log::Level::Trace, Self::info_acked::<P>(msg)));
        $buffer.map_mut(|buf| buf.remove(&msg.as_ref().map(|m| m.token)));
        Some(Ok($in))
      },
      _ => Some(Ok($in))
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
  use toad_msg::{Code, Payload};

  use super::*;
  use crate::platform::Effect;
  use crate::step::test::test_step;
  use crate::test;

  type InnerPollReq = Addrd<Req<test::Platform>>;
  type InnerPollResp = Addrd<Resp<test::Platform>>;
  type HandleAcks<S> = super::HandleAcks<S, BTreeMap<Addrd<Token>, ()>>;

  fn test_message(ty: Type) -> Addrd<test::Message> {
    use toad_msg::*;

    Addrd(test::Message { ver: Default::default(),
                          ty,
                          id: Id(1),
                          code: Code::new(0, 1),
                          token: Token(array_vec!(_ => 1)),
                          payload: Payload(Default::default()),
                          opts: Default::default() },
          test::dummy_addr())
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
      (on_message_sent(_, test::msg!(CON GET x.x.x.x:8080)) should satisfy { |out| assert_eq!(out, Err(Error::Inner(()))) })
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
      (inner.poll_req => { Some(Ok(test::msg!(ACK {0 . 01} x.x.x.x:8080).map(Req::from))) }),
      (inner.poll_resp => { Some(Ok(test::msg!(ACK {2 . 05} x.x.x.x:8080).map(Resp::from))) }),
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

  #[test]
  fn when_expected_piggybacked_ack_received_it_should_be_processed_and_returned() {
    struct TestState {
      token_last_sent: Option<Token>,
    }

    type Mock =
      test::MockStep<TestState, Addrd<Req<test::Platform>>, Addrd<Resp<test::Platform>>, ()>;

    let sut = HandleAcks::<Mock>::default();

    sut.inner()
       .init(TestState { token_last_sent: None })
       .set_on_message_sent(Box::new(|mock, _, msg| {
                              mock.state
                                  .map_mut(|s| {
                                    s.as_mut().unwrap().token_last_sent = Some(msg.data().token)
                                  });
                              Ok(())
                            }))
       .set_poll_resp(Box::new(|mock, _, _, poll_for_token, _| {
                        let mut msg = test::msg!(ACK {2 . 05} x.x.x.x:2222);

                        let token = mock.state
                                        .map_ref(|s| s.as_ref().unwrap().token_last_sent.unwrap());
                        Addrd::data_mut(&mut msg).token = token;

                        assert_eq!(token, poll_for_token);

                        let p = Payload(format!("oink oink!").bytes().collect::<Vec<_>>());
                        Addrd::data_mut(&mut msg).payload = p;

                        Some(Ok(msg.map(Resp::from)))
                      }));

    let token = Token(array_vec![1, 2, 3, 4]);

    let mut sent_req = test::msg!(CON GET x.x.x.x:2222);
    let dest = sent_req.addr();
    sent_req.as_mut().token = token;

    let snap = test::snapshot();
    let mut effs = Vec::<test::Effect>::new();

    sut.on_message_sent(&snap, &sent_req).unwrap();

    let res = sut.poll_resp(&snap, &mut effs, token, dest);
    assert!(!effs.is_empty());

    match &effs[0] {
      | Effect::Log(log::Level::Trace, msg) if msg.as_str().split(" ").next().unwrap() == "Got" => {
        ()
      },
      | e => panic!("{e:?}"),
    }

    assert_eq!(res.unwrap().unwrap().data().payload_string().unwrap(),
               format!("oink oink!"));
  }

  #[test]
  fn when_expected_empty_ack_received_it_should_be_processed_and_ignored() {
    struct TestState {
      token_last_sent: Option<Token>,
    }

    type Mock =
      test::MockStep<TestState, Addrd<Req<test::Platform>>, Addrd<Resp<test::Platform>>, ()>;

    let sut = HandleAcks::<Mock>::default();

    sut.inner()
       .init(TestState { token_last_sent: None })
       .set_on_message_sent(Box::new(|mock, _, msg| {
                              mock.state
                                  .map_mut(|s| {
                                    s.as_mut().unwrap().token_last_sent = Some(msg.data().token)
                                  });
                              Ok(())
                            }))
       .set_poll_resp(Box::new(|mock, _, _, poll_for_token, _| {
                        let mut msg = test::msg!(ACK {0 . 00} x.x.x.x:2222);

                        let token = mock.state
                                        .map_ref(|s| s.as_ref().unwrap().token_last_sent.unwrap());
                        Addrd::data_mut(&mut msg).token = token;

                        assert_eq!(token, poll_for_token);

                        Some(Ok(msg.map(Resp::from)))
                      }));

    let token = Token(array_vec![1, 2, 3, 4]);

    let mut sent_req = test::msg!(CON GET x.x.x.x:2222);
    let dest = sent_req.addr();
    sent_req.as_mut().token = token;

    let snap = test::snapshot();
    let mut effs = Vec::<test::Effect>::new();

    sut.on_message_sent(&snap, &sent_req).unwrap();

    let res = sut.poll_resp(&snap, &mut effs, token, dest);
    assert!(!effs.is_empty());

    match &effs[0] {
      | Effect::Log(log::Level::Warn, msg)
        if msg.as_str().split(" ").next().unwrap() == "DISCARDING" =>
      {
        ()
      },
      | e => panic!("{e:?}"),
    }

    assert_eq!(res, None);
  }
}
