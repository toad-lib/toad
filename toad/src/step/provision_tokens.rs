use embedded_time::Instant;
use no_std_net::SocketAddr;
use toad_common::Array;
use toad_msg::{CodeKind, Message, Token};

use super::{Step, StepOutput, _try};
use crate::config::Config;
use crate::net::Addrd;
use crate::platform::{self, Effect, Platform};
use crate::req::Req;
use crate::resp::Resp;
use crate::time::{self, Millis};
use crate::todo;

/// Errors that can be encountered when provisioning tokens
#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
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

impl<E> super::Error for Error<E> where E: super::Error {}

impl<E> From<E> for Error<E> {
  fn from(e: E) -> Self {
    Error::Inner(e)
  }
}

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
  fn next<E, Clock>(&mut self, now: Instant<Clock>, cfg: Config) -> Result<Token, Error<E>>
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
  for ProvisionTokens<Inner>
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
  type Error = Error<E>;
  type Inner = Inner;

  fn inner(&mut self) -> &mut Self::Inner {
    &mut self.inner
  }

  fn before_message_sent(&mut self,
                         snap: &platform::Snapshot<Dgram, Clock>,
                         msg: &mut Addrd<Message<MessagePayload, MessageOptions>>)
                         -> Result<(), Self::Error> {
    self.inner.before_message_sent(snap, msg)?;

    let token = match (msg.data().code.kind(), msg.data().token) {
      | (CodeKind::Request, t) if t == Token(Default::default()) => {
        self.next(snap.time, snap.config)?
      },
      | (_, t) => t,
    };

    msg.data_mut().token = token;

    Ok(())
  }

  fn poll_req(&mut self,
              snap: &platform::Snapshot<Dgram, Clock>,
              effects: &mut Effects)
              -> StepOutput<Self::PollReq, Self::Error> {
    self.inner
        .poll_req(snap, effects)
        .map(|r| r.map_err(|e| e.map(Error::Inner)))
  }

  fn poll_resp(&mut self,
               snap: &platform::Snapshot<Dgram, Clock>,
               effects: &mut Effects,
               token: Token,
               addr: SocketAddr)
               -> StepOutput<Self::PollResp, Self::Error> {
    self.inner
        .poll_resp(snap, effects, token, addr)
        .map(|r| r.map_err(|e| e.map(Error::Inner)))
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::req::ReqForPlatform;
  use crate::resp::RespForPlatform;
  use crate::step::test::test_step;
  use crate::test::{ClockMock, Snapshot};

  type InnerPollReq = Addrd<ReqForPlatform<crate::test::Platform>>;
  type InnerPollResp = Addrd<RespForPlatform<crate::test::Platform>>;

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
