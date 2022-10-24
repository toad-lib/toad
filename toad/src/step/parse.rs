use toad_msg::{CodeKind, TryFromBytes};

use super::{exec_inner_step, Step, StepOutput};
use crate::net::Addrd;
use crate::platform::{self, Platform};
use crate::req::Req;
use crate::resp::Resp;

/// TODO
#[derive(Debug, Clone, Copy)]
pub struct Parse<S>(S);

impl<S> Parse<S> {
  /// TODO
  pub fn new(s: S) -> Self {
    Self(s)
  }
}

/// TODO
#[derive(Clone, PartialEq)]
pub enum Error<E> {
  /// TODO
  Parsing(toad_msg::MessageParseError),
  /// TODO
  Inner(E),
}

impl<E: core::fmt::Debug> core::fmt::Debug for Error<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
          Self::Parsing(e) => f.debug_tuple("Parsing").field(e).finish(),
          Self::Inner(e) => e.fmt(f),
        }
    }
}

impl<E: super::Error> super::Error for Error<E> {}

macro_rules! common {
  ($dgram:expr) => {{
    $dgram.fold(|dgram, addr| {
                 platform::Message::<P>::try_from_bytes(dgram).map(|dgram| Addrd(dgram, addr))
               })
               .map_err(Error::Parsing)
               .map_err(nb::Error::Other)
  }};
}

impl<Inner: Step<P>, P: Platform> Step<P> for Parse<Inner> {
  type PollReq = Addrd<Req<P>>;
  type PollResp = Addrd<Resp<P>>;
  type Error = Error<Inner::Error>;

  fn poll_req(&mut self,
              snap: &crate::platform::Snapshot<P>,
              effects: &mut <P as Platform>::Effects)
              -> StepOutput<Self::PollReq, Error<Inner::Error>> {
    exec_inner_step!(self.0.poll_req(snap, effects), Error::Inner);
    let msg = common!(snap.recvd_dgram.as_ref());

    match msg {
      Ok(msg) if msg.data().code.kind() == CodeKind::Request => Some(Ok(msg.map(Req::from))),
      Ok(_) => None,
      Err(e) => Some(Err(e)),
    }
  }

  fn poll_resp(&mut self,
               snap: &crate::platform::Snapshot<P>,
               effects: &mut <P as Platform>::Effects,
               token: toad_msg::Token,
               addr: no_std_net::SocketAddr)
               -> StepOutput<Self::PollResp, Error<Inner::Error>> {
    exec_inner_step!(self.0.poll_resp(snap, effects, token, addr), Error::Inner);
    Some(common!(snap.recvd_dgram.as_ref())).map(|result| result.map(|addrd| addrd.map(Resp::from)))
  }
}

#[cfg(test)]
mod test {
  use embedded_time::Clock;
  use toad_msg::{Code, Type};

  use super::super::test;
  use super::{Error, Parse, Step};
  use crate::net::Addrd;
  use crate::platform;
  use crate::req::Req;

  fn test_msg(ty: Type, code: Code) -> (Addrd<Vec<u8>>, Addrd<Req<crate::test::Platform>>) {
    use toad_msg::*;

    let msg = platform::Message::<crate::test::Platform> { id: Id(1),
                                                           ty,
                                                           ver: Default::default(),
                                                           token: Token(Default::default()),
                                                           code,
                                                           opts: Default::default(),
                                                           payload: Payload(Default::default()) };

    (Addrd(msg.clone().try_into_bytes().unwrap(),
           crate::test::dummy_addr()),
     Addrd(Req::<_>::from(msg), crate::test::dummy_addr()))
  }

  test::test_step!(
      GIVEN
        this step { Parse::new }
        and inner step { impl Step<type Error = (), type PollReq = (), type PollResp = () }
        and io sequence { Default::default() }
        and snapshot default { test::default_snapshot() }
      WHEN
        poll_req is invoked
        and inner.poll_req returns error { Some(Err(nb::Error::Other(()))) }
      THEN
        poll_req should return error { Some(Err(nb::Error::Other(Error::Inner(())))) }
  );

  test::test_step!(
      GIVEN
        this step { Parse::new }
        and inner step { impl Step<type Error = (), type PollReq = (), type PollResp = () }
        and io sequence { Default::default() }
        and snapshot default { test::default_snapshot() }
      WHEN
        poll_req is invoked
        and inner.poll_req returns would_block { Some(Err(nb::Error::WouldBlock)) }
      THEN
        poll_req should return would_block { Some(Err(nb::Error::WouldBlock)) }
  );

  test::test_step!(
      GIVEN
        this step { Parse::new }
        and inner step { impl Step<type Error = (), type PollReq = (), type PollResp = () }
        and io sequence { Default::default() }
        and snapshot received_con_req {
          platform::Snapshot { time: crate::test::ClockMock::new().try_now().unwrap(),
                               recvd_dgram: test_msg(Type::Con, Code::new(1, 01)).0 }
        }
      WHEN
        poll_req is invoked
        and inner.poll_req returns nothing { None }
      THEN
        poll_req should return a_request { Some(Ok(test_msg(Type::Con, Code::new(1, 01)).1)) }
  );

  test::test_step!(
      GIVEN
        this step { Parse::new }
        and inner step { impl Step<type Error = (), type PollReq = (), type PollResp = () }
        and io sequence { Default::default() }
        and snapshot received_ack {
          platform::Snapshot { time: crate::test::ClockMock::new().try_now().unwrap(),
                               recvd_dgram: test_msg(Type::Ack, Code::new(0, 0)).0 }
        }
      WHEN
        poll_req is invoked
        and inner.poll_req returns nothing { None }
      THEN
        poll_req should return nothing { None }
  );

  test::test_step!(
      GIVEN
        this step { Parse::new }
        and inner step { impl Step<type Error = (), type PollReq = (), type PollResp = () }
        and io sequence { Default::default() }
        and snapshot received_response {
          platform::Snapshot { time: crate::test::ClockMock::new().try_now().unwrap(),
                               recvd_dgram: test_msg(Type::Ack, Code::new(2, 04)).0 }
        }
      WHEN
        poll_req is invoked
        and inner.poll_req returns nothing { None }
      THEN
        poll_req should return nothing { None }
  );
}
