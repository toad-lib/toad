use toad_common::Array;
use toad_msg::{CodeKind, Type};

use super::{exec_inner_step, Step, StepOutput};
use crate::net::Addrd;
use crate::platform::{Effect, PlatformTypes};
use crate::req::Req;
use crate::resp::Resp;

/// ACK incoming Confirmable messages
///
/// See the [module documentation](crate::step::ack) for more
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

type InnerPollReq<P> = Addrd<Req<P>>;
type InnerPollResp<P> = Addrd<Resp<P>>;

impl<Inner: Step<P, PollReq = InnerPollReq<P>, PollResp = InnerPollResp<P>>, P: PlatformTypes>
  Step<P> for Ack<Inner>
{
  type PollReq = Addrd<Req<P>>;
  type PollResp = Addrd<Resp<P>>;
  type Error = Inner::Error;
  type Inner = Inner;

  fn inner(&self) -> &Inner {
    &self.0
  }

  fn poll_req(&self,
              snap: &crate::platform::Snapshot<P>,
              effects: &mut <P as PlatformTypes>::Effects)
              -> StepOutput<Self::PollReq, Inner::Error> {
    match exec_inner_step!(self.0.poll_req(snap, effects), core::convert::identity) {
      | Some(req)
        if req.data().msg.ty == Type::Con && req.data().msg.code.kind() == CodeKind::Request =>
      {
        effects.push(Effect::Send(Addrd(Resp::ack(req.as_ref().data()).into(), req.addr())));
        Some(Ok(req))
      },
      | Some(req) => Some(Ok(req)),
      | None => None,
    }
  }

  fn poll_resp(&self,
               snap: &crate::platform::Snapshot<P>,
               effects: &mut <P as PlatformTypes>::Effects,
               token: toad_msg::Token,
               addr: no_std_net::SocketAddr)
               -> StepOutput<Self::PollResp, Inner::Error> {
    exec_inner_step!(self.0.poll_resp(snap, effects, token, addr),
                     core::convert::identity).map(Ok)
  }
}

#[cfg(test)]
mod test {
  use toad_msg::{Code, Type};

  use super::super::test;
  use super::{Ack, Effect, Step};
  use crate::net::Addrd;
  use crate::platform;
  use crate::req::Req;
  use crate::resp::Resp;

  type InnerPollReq = super::InnerPollReq<crate::test::Platform>;
  type InnerPollResp = super::InnerPollResp<crate::test::Platform>;

  fn test_msg(ty: Type,
              code: Code)
              -> (Addrd<Req<crate::test::Platform>>, Addrd<Resp<crate::test::Platform>>) {
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
        (inner.poll_req => { Some(Ok(test_msg(Type::Con, Code::new(0, 01)).0)) })
      ]
      THEN poll_req_should_ack [
        (poll_req(_, _) should satisfy { |out| assert_eq!(out, Some(Ok(test_msg(Type::Con, Code::new(0, 01)).0))) }),
        (effects == {
          vec![
            Effect::Send(
              Addrd(
                Resp::ack(&test_msg(Type::Con, Code::new(1, 01)).0.0)
                  .into(),
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
