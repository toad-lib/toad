use core::fmt::Write;

use tinyvec::ArrayVec;
use toad_msg::MessageOptions;
use toad_writable::Writable;

use super::{Step, StepOutput};
use crate::net::Addrd;
use crate::platform;
use crate::platform::PlatformTypes;
use crate::req::Req;
use crate::resp::Resp;

/// Struct responsible for buffering and yielding responses to the request
/// we're polling for.
///
/// For more information, see the [module documentation](crate::step::buffer_responses).
#[derive(Debug)]
pub struct SetStandardOptions<S>(S);

impl<S> Default for SetStandardOptions<S> where S: Default
{
  fn default() -> Self {
    Self(S::default())
  }
}

impl<P, E, S> Step<P> for SetStandardOptions<S>
  where P: PlatformTypes,
        E: super::Error,
        S: Step<P, PollReq = Addrd<Req<P>>, PollResp = Addrd<Resp<P>>, Error = E>
{
  type PollReq = Addrd<Req<P>>;
  type PollResp = Addrd<Resp<P>>;
  type Error = E;
  type Inner = S;

  fn inner(&self) -> &S {
    &self.0
  }

  fn poll_req(&self,
              snap: &crate::platform::Snapshot<P>,
              effects: &mut <P as PlatformTypes>::Effects)
              -> StepOutput<Self::PollReq, Self::Error> {
    self.0.poll_req(snap, effects)
  }

  fn poll_resp(&self,
               snap: &crate::platform::Snapshot<P>,
               effects: &mut <P as PlatformTypes>::Effects,
               token: toad_msg::Token,
               addr: no_std_net::SocketAddr)
               -> StepOutput<Self::PollResp, Self::Error> {
    self.0.poll_resp(snap, effects, token, addr)
  }

  fn before_message_sent(&self,
                         snap: &platform::Snapshot<P>,
                         effs: &mut P::Effects,
                         msg: &mut Addrd<platform::Message<P>>)
                         -> Result<(), Self::Error> {
    self.0.before_message_sent(snap, effs, msg)?;

    let (host, port) = (msg.addr().ip(), msg.addr().port());

    let mut bytes = Writable::<ArrayVec<[u8; 4]>>::default();
    write!(bytes, "{}", host).ok();
    msg.as_mut().set_host(bytes.as_str()).ok();
    msg.as_mut().set_port(port).ok();

    let payload_len = msg.data().payload().as_bytes().len() as u64;
    match msg.data().code.kind() {
      | toad_msg::CodeKind::Request => {
        msg.as_mut().set_size1(payload_len).ok();
      },
      | toad_msg::CodeKind::Response => {
        msg.as_mut().set_size2(payload_len).ok();
      },
      | toad_msg::CodeKind::Empty => (),
    }

    Ok(())
  }
}

#[cfg(test)]
mod test {
  use embedded_time::Instant;
  use tinyvec::array_vec;
  use toad_msg::Type;

  use super::*;
  use crate::platform::Snapshot;
  use crate::step::test::test_step;
  use crate::test;

  type InnerPollReq = Addrd<Req<test::Platform>>;
  type InnerPollResp = Addrd<Resp<test::Platform>>;

  fn test_message(ty: Type) -> Addrd<test::Message> {
    use toad_msg::*;

    Addrd(test::Message { ver: Default::default(),
                          ty,
                          id: Id(1),
                          code: Code::new(1, 1),
                          token: Token(array_vec!(_ => 1)),
                          payload: Payload(Default::default()),
                          opts: Default::default() },
          test::dummy_addr())
  }

  test_step!(
    GIVEN SetStandardOptions::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
    WHEN inner_errors [
      (inner.poll_req => { Some(Err(nb::Error::Other(()))) }),
      (inner.poll_resp => { Some(Err(nb::Error::Other(()))) }),
      (inner.on_message_sent = { |_, _| Err(()) })
    ]
    THEN this_should_error [
      (poll_req(_, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::Other(())))) }),
      (poll_resp(_, _, _, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::Other(())))) }),
      (on_message_sent(_, test_message(Type::Con)) should satisfy { |out| assert_eq!(out, Err(())) })
    ]
  );

  test_step!(
    GIVEN SetStandardOptions::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
    WHEN inner_blocks [
      (inner.poll_req => { Some(Err(nb::Error::WouldBlock)) }),
      (inner.poll_resp => { Some(Err(nb::Error::WouldBlock)) })
    ]
    THEN this_should_block [
      (poll_req(_, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::WouldBlock))) }),
      (poll_resp(_, _, _, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::WouldBlock))) })
    ]
  );

  #[test]
  fn options() {
    crate::step::test::dummy_step!({Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>});
    let s = SetStandardOptions::<Dummy>::default();
    let snap = Snapshot { time: Instant::new(0),
                          config: Default::default(),
                          recvd_dgram: None };

    let mut req = test::msg!(CON GET x.x.x.x:80);
    req.as_mut().payload = toad_msg::Payload("Yabba dabba doo!!".bytes().collect());

    let mut resp = test::msg!(CON {2 . 04} x.x.x.x:80);
    resp.as_mut().payload =
      toad_msg::Payload("wacky tobaccy is the smacky holacky".bytes().collect());

    s.before_message_sent(&snap, &mut vec![], &mut req).unwrap();
    s.before_message_sent(&snap, &mut vec![], &mut resp)
     .unwrap();
    assert_eq!(req.data().size1(), Some(17));
    assert_eq!(resp.data().size2(), Some(35));
  }
}
