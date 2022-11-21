use toad_common::Array;
use toad_msg::to_bytes::MessageToBytesError;
use toad_msg::{CodeKind, TryIntoBytes, Type};

use super::{exec_inner_step, Step, StepOutput, _try};
use crate::net::Addrd;
use crate::platform::{Effect, Platform, Snapshot};
use crate::req::{Req, ReqForPlatform};
use crate::resp::{Resp, RespForPlatform};
use crate::{time, todo};

/// The message parsing CoAP lifecycle step
///
/// Parameterized by the step that came before it,
/// most likely this is the [`Empty`](crate::step::Empty) step.
///
/// See the [module documentation](crate::step::parse) for more.
#[derive(Debug, Clone, Copy)]
pub struct Ack<S>(S);

impl<S: Default> Default for Ack<S> {
  fn default() -> Self {
    Ack(Default::default())
  }
}

impl<S> Ack<S> {
  /// Create a new Ack step
  pub fn new(s: S) -> Self {
    Self(s)
  }
}

type InnerPollReq<P> = Addrd<ReqForPlatform<P>>;
type InnerPollResp<P> = Addrd<RespForPlatform<P>>;

impl<Dgram,
      Inner,
      E,
      Effects,
      MessagePayload,
      MessageOptionValue,
      MessageOptions,
      NumberedOptions,
      Clock>
  Step<Dgram, Effects, MessagePayload, MessageOptionValue, MessageOptions, NumberedOptions, Clock>
  for Ack<Inner>
  where Dgram: crate::net::Dgram,
        E: super::Error,
        Inner: Step<Dgram,
                    Effects,
                    MessagePayload,
                    MessageOptionValue,
                    MessageOptions,
                    NumberedOptions,
                    Clock,
                    PollReq = Addrd<Req<MessagePayload,
                                        MessageOptionValue,
                                        MessageOptions,
                                        NumberedOptions>>,
                    PollResp = Addrd<Resp<MessagePayload,
                                          MessageOptionValue,
                                          MessageOptions,
                                          NumberedOptions>>,
                    Error = E>,
        Effects: Array<Item = Effect<MessagePayload, MessageOptions>>,
        MessagePayload: todo::MessagePayload,
        MessageOptionValue: todo::MessageOptionValue,
        MessageOptions: todo::MessageOptions<MessageOptionValue>,
        NumberedOptions: todo::NumberedOptions<MessageOptionValue>,
        Clock: time::Clock
{
  type PollReq = Addrd<Req<MessagePayload, MessageOptionValue, MessageOptions, NumberedOptions>>;
  type PollResp = Addrd<Resp<MessagePayload, MessageOptionValue, MessageOptions, NumberedOptions>>;
  type Error = E;
  type Inner = Inner;

  fn inner(&mut self) -> &mut Inner {
    &mut self.0
  }

  fn poll_req(&mut self,
              snap: &Snapshot<Dgram, Clock>,
              effects: &mut Effects)
              -> StepOutput<Self::PollReq, Self::Error> {
    let req = _try!(Option<nb::Result>; self.0.poll_req(snap, effects));

    if req.data().msg.ty == Type::Con && req.data().msg.code.kind() == CodeKind::Request {
      effects.push(Effect::SendMessage(Addrd(Resp::ack(req.as_ref().data()).into(), req.addr())));
    }

    Some(Ok(req))
  }

  fn poll_resp(&mut self,
               snap: &Snapshot<Dgram, Clock>,
               effects: &mut Effects,
               token: toad_msg::Token,
               addr: no_std_net::SocketAddr)
               -> StepOutput<Self::PollResp, Self::Error> {
    self.0.poll_resp(snap, effects, token, addr)
  }
}

#[cfg(test)]
mod test {
  use toad_msg::{Code, Type};

  use super::super::test;
  use super::{Ack, Effect, Step, TryIntoBytes};
  use crate::net::Addrd;
  use crate::platform;
  use crate::req::{Req, ReqForPlatform};
  use crate::resp::{Resp, RespForPlatform};

  type InnerPollReq = super::InnerPollReq<crate::test::Platform>;
  type InnerPollResp = super::InnerPollResp<crate::test::Platform>;

  fn test_msg(
    ty: Type,
    code: Code)
    -> (Addrd<ReqForPlatform<crate::test::Platform>>, Addrd<RespForPlatform<crate::test::Platform>>)
  {
    use toad_msg::*;

    type Msg = platform::Message<crate::test::Platform>;
    let msg = Msg { id: Id(1),
                    ty,
                    ver: Default::default(),
                    token: Token(Default::default()),
                    code,
                    opts: Default::default(),
                    payload: Payload(Default::default()) };

    let addr = crate::test::dummy_addr();

    (Addrd(Req::<_>::from(msg.clone()), addr), Addrd(Resp::<_>::from(msg), addr))
  }

  test::test_step!(
      GIVEN Ack::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
      WHEN inner_errors [
        (inner.poll_req => { Some(Err(nb::Error::Other(()))) }),
        (inner.poll_resp => { Some(Err(nb::Error::Other(()))) })
      ]
      THEN this_should_error [
        (poll_req(_, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::Other(())))) }),
        (poll_resp(_, _, _, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::Other(())))) })
      ]
  );

  test::test_step!(
      GIVEN Ack::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
      WHEN inner_blocks [
        (inner.poll_req => { Some(Err(nb::Error::WouldBlock)) }),
        (inner.poll_resp => { Some(Err(nb::Error::WouldBlock)) })
      ]
      THEN this_should_block [
        (poll_req(_, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::WouldBlock))) }),
        (poll_resp(_, _, _, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::WouldBlock))) })
      ]
  );

  test::test_step!(
      GIVEN Ack::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
      WHEN inner_yields_non_request [
        (inner.poll_req => { Some(Ok(test_msg(Type::Non, Code::new(1, 01)).0)) })
      ]
      THEN poll_req_should_noop [
        (poll_req(_, _) should satisfy { |out| assert_eq!(out, Some(Ok(test_msg(Type::Non, Code::new(1, 01)).0))) }),
        (effects == { vec![] })
      ]
  );

  test::test_step!(
      GIVEN Ack::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
      WHEN inner_yields_response [
        (inner.poll_req => { Some(Ok(test_msg(Type::Ack, Code::new(0, 00)).0)) })
      ]
      THEN poll_req_should_noop [
        (poll_req(_, _) should satisfy { |out| assert_eq!(out, Some(Ok(test_msg( Type::Ack, Code::new(0, 00)).0))) }),
        (effects == { vec![] })
      ]
  );

  test::test_step!(
      GIVEN Ack::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
      WHEN inner_yields_con_request [
        (inner.poll_req => { Some(Ok(test_msg(Type::Con, Code::new(1, 01)).0)) })
      ]
      THEN poll_req_should_ack [
        (poll_req(_, _) should satisfy { |out| assert_eq!(out, Some(Ok(test_msg(Type::Con, Code::new(1, 01)).0))) }),
        (effects == {
          vec![
            Effect::SendDgram(
              Addrd(
                Resp::ack(&test_msg(Type::Con, Code::new(1, 01)).0.0)
                  .try_into_bytes()
                  .unwrap(),
                crate::test::dummy_addr()
              )
            )
          ]
        })
      ]
  );

  test::test_step!(
      GIVEN Ack::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
      WHEN inner_yields_anything [
        (inner.poll_resp => { Some(Ok(test_msg(Type::Ack, Code::new(2, 04)).1)) })
      ]
      THEN poll_resp_should_noop [
        (poll_resp(_, _, _, _) should satisfy { |out| assert_eq!(out, Some(Ok(test_msg(Type::Ack, Code::new(2, 04)).1))) }),
        (effects == { vec![] })
      ]
  );
}
