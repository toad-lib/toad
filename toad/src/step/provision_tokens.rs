use embedded_time::Instant;
use no_std_net::SocketAddr;
use toad_msg::{CodeKind, Token};

use super::Step;
use crate::config::Config;
use crate::net::Addrd;
use crate::platform;
use crate::platform::Platform;
use crate::req::Req;
use crate::resp::Resp;
use crate::time::Millis;

/// Step responsible for replacing all message ids of zero `Id(0)` (assumed to be meaningless)
/// with a new meaningful Id that is guaranteed to be unique to the conversation with
/// the message's origin/destination address.
#[derive(Debug, Clone)]
pub struct ProvisionTokens<Inner> {
  inner: Inner,
}

impl<Inner> Default for ProvisionTokens<Inner> where Inner: Default
{
  fn default() -> Self {
    Self { inner: Default::default() }
  }
}

impl<Inner> ProvisionTokens<Inner> {
  fn next<Clock>(&mut self, now: Instant<Clock>, cfg: Config) -> Token
    where Clock: crate::time::Clock
  {
    let now_since_epoch = Millis::try_from(now.duration_since_epoch()).unwrap();

    #[allow(clippy::many_single_char_names)]
    let bytes = {
      let ([a, b], [c, d, e, f, g, h, i, j]) =
        (cfg.msg.token_seed.to_be_bytes(), now_since_epoch.0.to_be_bytes());
      [a, b, c, d, e, f, g, h, i, j]
    };

    Token::opaque(&bytes)
  }
}

impl<P, E: super::Error, Inner> Step<P> for ProvisionTokens<Inner>
  where P: Platform,
        Inner: Step<P, PollReq = Addrd<Req<P>>, PollResp = Addrd<Resp<P>>, Error = E>
{
  type PollReq = Addrd<Req<P>>;
  type PollResp = Addrd<Resp<P>>;
  type Error = E;
  type Inner = Inner;

  fn inner(&mut self) -> &mut Self::Inner {
    &mut self.inner
  }

  fn before_message_sent(&mut self,
                         snap: &platform::Snapshot<P>,
                         msg: &mut Addrd<platform::Message<P>>)
                         -> Result<(), Self::Error> {
    self.inner.before_message_sent(snap, msg)?;

    let token = match (msg.data().code.kind(), msg.data().token) {
      | (CodeKind::Request, t) if t == Token(Default::default()) => {
        self.next(snap.time, snap.config)
      },
      | (_, t) => t,
    };

    msg.data_mut().token = token;

    Ok(())
  }

  fn poll_req(&mut self,
              snap: &platform::Snapshot<P>,
              effects: &mut <P as Platform>::Effects)
              -> super::StepOutput<Self::PollReq, Self::Error> {
    self.inner.poll_req(snap, effects)
  }

  fn poll_resp(&mut self,
               snap: &platform::Snapshot<P>,
               effects: &mut <P as Platform>::Effects,
               token: Token,
               addr: SocketAddr)
               -> super::StepOutput<Self::PollResp, Self::Error> {
    self.inner.poll_resp(snap, effects, token, addr)
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::step::test::test_step;
  use crate::test::{ClockMock, Snapshot};

  type InnerPollReq = Addrd<Req<crate::test::Platform>>;
  type InnerPollResp = Addrd<Resp<crate::test::Platform>>;

  test_step!(
    GIVEN ProvisionTokens::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
    WHEN inner_errors [
      (inner.poll_req => { Some(Err(nb::Error::Other(()))) }),
      (inner.poll_resp => { Some(Err(nb::Error::Other(()))) })
    ]
    THEN this_should_error [
      (poll_req(_, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::Other(())))) }),
      (poll_resp(_, _, _, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::Other(())))) })
    ]
  );

  test_step!(
    GIVEN ProvisionTokens::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
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
    GIVEN ProvisionTokens::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
    WHEN we_boutta_send_a_request [
      (inner.before_message_sent = { |_, _| Ok(()) })
    ]
    THEN this_should_make_sure_it_has_a_token [
      (before_message_sent(
          Snapshot { time: ClockMock::instant(0),
                     recvd_dgram: Addrd(Default::default(), crate::test::dummy_addr()),
                     config: Config::default() },
          crate::test::msg!(CON GET x.x.x.x:80)
      ) should satisfy { |m| assert_ne!(m.data().token, Token(Default::default())) })
    ]
  );

  test_step!(
    GIVEN ProvisionTokens::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
    WHEN we_boutta_send_a_response [
      (inner.before_message_sent = { |_, _| Ok(()) })
    ]
    THEN this_should_make_sure_it_has_a_token [
      (before_message_sent(
          Snapshot { time: ClockMock::instant(0),
                     recvd_dgram: Addrd(Default::default(), crate::test::dummy_addr()),
                     config: Config::default() },
          crate::test::msg!(CON {2 . 04} x.x.x.x:80)
      ) should satisfy { |m| assert_eq!(m.data().token, Token(Default::default())) })
    ]
  );
}
