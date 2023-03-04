use embedded_time::Instant;
use no_std_net::SocketAddr;
use toad_msg::{CodeKind, Token};

use super::Step;
use crate::config::Config;
use crate::net::Addrd;
use crate::platform;
use crate::platform::PlatformTypes;
use crate::req::Req;
use crate::resp::Resp;
use crate::time::Millis;

/// Errors that can be encountered when provisioning tokens
#[derive(PartialEq, Eq, PartialOrd, Clone, Copy)]
pub enum Error<E> {
  /// The inner step failed.
  ///
  /// This variant's Debug representation is completely
  /// replaced by the inner type E's debug representation.
  Inner(E),
  /// This exceedingly rare error will only ever happen
  /// when the [`Clock`](crate::time::Clock) implementation
  /// is defined as 1 tick meaning 1 second.
  ///
  /// If this is the case, it would be highly advised to use
  /// milli ticks, as seconds are too granular to be reliable
  /// for timings used in `toad`.
  MillisSinceEpochWouldOverflow,
}

impl<E: core::fmt::Debug> core::fmt::Debug for Error<E> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    match self {
      | Self::MillisSinceEpochWouldOverflow => {
        f.debug_tuple("MillisSinceEpochWouldOverflow").finish()
      },
      | Self::Inner(e) => e.fmt(f),
    }
  }
}

impl<E> super::Error for Error<E> where E: super::Error {}

impl<E> From<E> for Error<E> {
  fn from(e: E) -> Self {
    Error::Inner(e)
  }
}

/// Step responsible for setting the token of all outbound messages with
/// empty tokens (`Token(Default::default())`, assumed to be meaningless)
/// with a new token that is guaranteed to be unique to the conversation with
/// the message's origin/destination address.
///
/// For more information, see the [module documentation](crate::step::provision_tokens).
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
  fn next<E, Clock>(&self, now: Instant<Clock>, cfg: Config) -> Result<Token, Error<E>>
    where Clock: crate::time::Clock
  {
    // TODO(orion): we may want to handle this
    let now_since_epoch =
      Millis::try_from(now.duration_since_epoch()).map_err(|_| {
                                                    Error::MillisSinceEpochWouldOverflow
                                                  })?;

    #[allow(clippy::many_single_char_names)]
    let bytes = {
      let ([a, b], [c, d, e, f, g, h, i, j]) =
        (cfg.msg.token_seed.to_be_bytes(), now_since_epoch.0.to_be_bytes());
      [a, b, c, d, e, f, g, h, i, j]
    };

    Ok(Token::opaque(&bytes))
  }
}

impl<P, E: super::Error, Inner> Step<P> for ProvisionTokens<Inner>
  where P: PlatformTypes,
        Inner: Step<P, PollReq = Addrd<Req<P>>, PollResp = Addrd<Resp<P>>, Error = E>
{
  type PollReq = Addrd<Req<P>>;
  type PollResp = Addrd<Resp<P>>;
  type Error = Error<E>;
  type Inner = Inner;

  fn inner(&self) -> &Inner {
    &self.inner
  }

  fn before_message_sent(&self,
                         snap: &platform::Snapshot<P>,
                         effs: &mut P::Effects,
                         msg: &mut Addrd<platform::Message<P>>)
                         -> Result<(), Self::Error> {
    self.inner.before_message_sent(snap, effs, msg)?;

    let token = match (msg.data().code.kind(), msg.data().token) {
      | (CodeKind::Request, t) if t == Token(Default::default()) => {
        self.next(snap.time, snap.config)?
      },
      | (_, t) => t,
    };

    msg.data_mut().token = token;

    Ok(())
  }

  fn poll_req(&self,
              snap: &platform::Snapshot<P>,
              effects: &mut <P as PlatformTypes>::Effects)
              -> super::StepOutput<Self::PollReq, Self::Error> {
    self.inner
        .poll_req(snap, effects)
        .map(|r| r.map_err(|e| e.map(Error::Inner)))
  }

  fn poll_resp(&self,
               snap: &platform::Snapshot<P>,
               effects: &mut <P as PlatformTypes>::Effects,
               token: Token,
               addr: SocketAddr)
               -> super::StepOutput<Self::PollResp, Self::Error> {
    self.inner
        .poll_resp(snap, effects, token, addr)
        .map(|r| r.map_err(|e| e.map(Error::Inner)))
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
      (poll_req(_, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::Other(Error::Inner(()))))) }),
      (poll_resp(_, _, _, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::Other(Error::Inner(()))))) })
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
      (inner.before_message_sent = { |_, _, _| Ok(()) })
    ]
    THEN this_should_make_sure_it_has_a_token [
      (before_message_sent(
          Snapshot { time: ClockMock::instant(0),
                     recvd_dgram: Some(Addrd(Default::default(), crate::test::dummy_addr())),
                     config: Config::default() },
                     _,
          crate::test::msg!(CON GET x.x.x.x:80)
      ) should satisfy { |m| assert_ne!(m.data().token, Token(Default::default())) })
    ]
  );

  test_step!(
    GIVEN ProvisionTokens::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
    WHEN we_boutta_send_a_response [
      (inner.before_message_sent = { |_, _, _| Ok(()) })
    ]
    THEN this_should_make_sure_it_has_a_token [
      (before_message_sent(
          Snapshot { time: ClockMock::instant(0),
                     recvd_dgram: Some(Addrd(Default::default(), crate::test::dummy_addr())),
                     config: Config::default() },
                     _,
          crate::test::msg!(CON {2 . 04} x.x.x.x:80)
      ) should satisfy { |m| assert_eq!(m.data().token, Token(Default::default())) })
    ]
  );
}
