use core::fmt::Write;

use tinyvec::ArrayVec;
use toad_common::Writable;
use toad_msg::MessageOptions;

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

    Ok(())
  }
}

#[cfg(test)]
mod test {
  use tinyvec::array_vec;
  use toad_msg::Type;

  use super::*;
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
}
