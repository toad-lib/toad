use embedded_time::duration::Milliseconds;
use embedded_time::Instant;
use toad_array::Array;
use toad_msg::{CodeKind, Token, Type};
use toad_stem::Stem;
use toad_string::{format, String};

use super::{log, Step, StepOutput, _try};
use crate::config::Config;
use crate::net::Addrd;
use crate::platform::{self, Effect, PlatformTypes, Snapshot};
use crate::req::Req;
use crate::resp::Resp;
use crate::retry::{Attempts, RetryTimer, Strategy, YouShould};
use crate::time::{Clock, Millis};

#[allow(missing_docs)]
#[allow(missing_debug_implementations)]
#[allow(missing_copy_implementations)]
pub struct Debug {
  pub msg_short: String<100>,
  pub msg_should_be: String<16>,
  pub since_last_attempt: Millis,
  pub since_first_attempt: Millis,
  pub until_next_attempt: Option<Millis>,
}

/// Buffer used to store messages queued for retry
pub trait Buf<P>
  where P: PlatformTypes,
        Self: Array<Item = (State<P::Clock>, Addrd<platform::Message<P>>)>
{
  /// Data points used by log messaging
  fn debug(now: Instant<P::Clock>,
           state: &State<P::Clock>,
           msg: &Addrd<platform::toad_msg::Message<P>>)
           -> Debug {
    let msg_short = format!(100,
                            "{:?} {:?} {:?}",
                            msg.data().ty,
                            msg.data().code,
                            msg.data().token);
    let since_first_attempt = Millis::try_from(now - state.retry_timer().first_attempted_at()).expect("duration since first attempt should be less than u64::MAX milliseconds");
    let since_last_attempt = Millis::try_from(now - state.retry_timer().last_attempted_at()).expect("duration since last attempt should be less than u64::MAX milliseconds");
    let until_next_attempt = state.retry_timer().next_attempt_at().checked_duration_since(&now).map(|until| Millis::try_from(until).expect("duration until next attempt should be less than u64::MAX milliseconds"));
    let msg_should_be = if msg.data().ty == Type::Con {
                          "acknowledged"
                        } else {
                          "responded to"
                        }.into();
    Debug { since_first_attempt,
            since_last_attempt,
            until_next_attempt,
            msg_should_be,
            msg_short }
  }

  /// Send all messages that need to be sent
  fn attempt_all<E>(&mut self,
                    now: Instant<P::Clock>,
                    effects: &mut P::Effects)
                    -> Result<(), Error<E>> {
    self.iter_mut().for_each(|(state, msg)| {
                     let dbg = Self::debug(now, state, msg);
                     match state.timer().what_should_i_do(now) {
                       | Ok(YouShould::Retry) => {
                         log!(retry::Buf::attempt_all,
                              effects,
                              log::Level::Info,
                              "{} not {} in {}ms. retrying...",
                              dbg.msg_short,
                              dbg.msg_should_be,
                              dbg.since_last_attempt);
                         effects.push(Effect::Send(msg.clone()));
                       },
                       | _ => log!(retry::Buf::attempt_all,
                                   effects,
                                   log::Level::Trace,
                                   "{} not {} in {}ms, will retry in {:?}",
                                   dbg.msg_short,
                                   dbg.msg_should_be,
                                   dbg.since_last_attempt,
                                   dbg.until_next_attempt),
                     }
                   });
    Ok(())
  }

  /// We saw a response and should remove all tracking of a token (if we have any)
  fn forget(&mut self, now: Instant<P::Clock>, effects: &mut P::Effects, token: Token) {
    match self.iter()
              .enumerate()
              .find(|(_, (_, msg))| msg.data().token == token)
    {
      | Some((ix, (state, msg))) => {
        let dbg = Self::debug(now, state, msg);
        log!(retry::Buf::forget,
             effects,
             log::Level::Debug,
             "{} {} after waiting {}ms since last attempt (first attempt {}ms ago)",
             dbg.msg_short,
             dbg.msg_should_be,
             dbg.since_last_attempt,
             dbg.since_first_attempt);
        self.remove(ix);
      },
      | _ => (),
    }
  }

  /// We saw an ACK and should transition the retry state for matching outbound
  /// CONs to the "acked" state
  fn mark_acked(&mut self, now: Instant<P::Clock>, effects: &mut P::Effects, token: Token) {
    let found = self.iter_mut().find(|(_, msg)| msg.data().token == token);

    match found {
      | Some((_, msg)) if msg.data().code.kind() == CodeKind::Response => {
        self.forget(now, effects, token);
      },
      | Some((state, msg)) if matches!(state, State::ConPreAck { .. }) => {
        let dbg = Self::debug(now, state, msg);
        log!(retry::Buf::mark_acked,
             effects,
             log::Level::Debug,
             "{} request acked after waiting {}ms since last attempt (first attempt {}ms ago)",
             dbg.msg_short,
             dbg.since_last_attempt,
             dbg.since_first_attempt);

        let timer = match state {
          | State::ConPreAck { post_ack_strategy,
                               post_ack_max_attempts,
                               .. } => {
            RetryTimer::new(now, *post_ack_strategy, *post_ack_max_attempts)
          },
          | _ => unreachable!(),
        };

        *state = State::Just(timer);
      },
      | _ => {
        log!(retry::Buf::mark_acked,
             effects,
             log::Level::Info,
             "ACK {:?} does not apply to any known messages",
             token);
      },
    };
  }

  /// We saw a RESET regarding token `token`
  fn mark_reset(&mut self, now: Instant<P::Clock>, effects: &mut P::Effects, token: Token) {
    let found = self.iter().find(|(_, msg)| msg.data().token == token);

    match found {
      | Some((state, msg)) => {
        let dbg = Self::debug(now, state, msg);
        log!(retry::Buf::mark_reset,
             effects,
             log::Level::Debug,
             "{} got RESET, dropping all retry state.",
             dbg.msg_short);
        self.forget(now, effects, token)
      },
      | _ => {
        log!(retry::Buf::mark_reset,
             effects,
             log::Level::Info,
             "RESET {:?} does not correspond to any known messages",
             token);
      },
    };
  }

  /// Called when a response of any kind to any request is
  /// received
  ///
  /// May invoke `mark_acked` & `forget`
  fn maybe_seen_response<E>(&mut self,
                            now: Instant<P::Clock>,
                            effects: &mut P::Effects,
                            msg: Addrd<&platform::Message<P>>)
                            -> Result<(), Error<E>> {
    match (msg.data().ty, msg.data().code.kind()) {
      | (Type::Reset, _) => {
        self.mark_reset(now, effects, msg.data().token);
        Ok(())
      },
      | (Type::Ack, CodeKind::Empty) => {
        log!(retry::Buf::maybe_seen_response, effects, log::Level::Trace, "ACK 0.00 {:?} means we should find the corresponding outbound CON and either forget (if CON response) or transition to expecting a response (if CON request). No following logs means the ACK was unexpected.", msg.data().token);
        self.mark_acked(now, effects, msg.data().token);
        Ok(())
      },
      | (_, CodeKind::Response) => {
        log!(retry::Buf::maybe_seen_response, effects, log::Level::Trace, "{:?} {:?} {:?} means we should find and forget the originating request. No following logs means the response was unexpected.", msg.data().ty, msg.data().code, msg.data().token);
        self.forget(now, effects, msg.data().token);
        Ok(())
      },
      | _ => {
        log!(retry::Buf::maybe_seen_response,
             effects,
             log::Level::Trace,
             "ignoring a {:?} {:?} message; could not possibly affect any pending retries",
             msg.data().ty,
             msg.data().code);
        Ok(())
      },
    }
  }

  /// Called when a message of any kind is sent,
  /// and may store it to be retried in the future
  fn store_retryables<E>(&mut self,
                         now: Instant<P::Clock>,
                         effects: &mut P::Effects,
                         msg: &Addrd<platform::Message<P>>,
                         config: Config)
                         -> Result<(), Error<E>> {
    match msg.data().ty {
      | Type::Con | Type::Non if self.is_full() => Err(Error::RetryBufferFull),
      | Type::Con => {
        let timer = RetryTimer::new(now,
                                    config.msg.con.unacked_retry_strategy,
                                    config.msg.con.max_attempts);
        self.push((State::ConPreAck { timer,
                                      post_ack_strategy: config.msg.con.acked_retry_strategy,
                                      post_ack_max_attempts: config.msg.con.max_attempts },
                   msg.clone()));

        log!(retry::Buf::store_retryables,
             effects,
             log::Level::Trace,
             "sent CON {:?}; will retry if no ACK",
             msg.data().code);

        Ok(())
      },
      | Type::Non if msg.data().code.kind() == CodeKind::Request => {
        log!(retry::Buf::store_retryables,
             effects,
             log::Level::Trace,
             "sent NON request {:?}; will retry if no response",
             msg.data().code);
        let timer = RetryTimer::new(now,
                                    config.msg.non.retry_strategy,
                                    config.msg.non.max_attempts);
        self.push((State::Just(timer), msg.clone()));

        Ok(())
      },
      | _ => {
        log!(retry::Buf::store_retryables,
             effects,
             log::Level::Trace,
             "{:?} {:?} will not be retried",
             msg.data().ty,
             msg.data().code);
        Ok(())
      },
    }
  }
}

impl<T, P> Buf<P> for T
  where T: Array<Item = (State<P::Clock>, Addrd<platform::Message<P>>)>,
        P: PlatformTypes
{
}

/// The state of a message stored in the retry [buffer](Buf)
#[derive(PartialEq, Eq, Debug)]
pub enum State<C>
  where C: Clock
{
  /// A message that is not an un-acked CON
  ///
  /// (meaning the retry strategy will never change)
  Just(RetryTimer<C>),
  /// A message that is an un-acked CON
  ///
  /// This means that when it is acked,
  /// we will need to replace the current
  /// retry timer with one using the
  /// [acked CON retry strategy](crate::config::Con.acked_retry_strategy).
  ConPreAck {
    /// The current (unacked) retry state
    timer: RetryTimer<C>,
    /// The strategy to use once the message is acked
    post_ack_strategy: Strategy,
    /// The max number of retry attempts for the post-ack state
    post_ack_max_attempts: Attempts,
  },
}

impl<C> State<C> where C: Clock
{
  /// Gets the current in-use retry timer
  pub fn retry_timer(&self) -> &RetryTimer<C> {
    match self {
      | Self::Just(r) => r,
      | Self::ConPreAck { timer, .. } => timer,
    }
  }
}

impl<C> Copy for State<C> where C: Clock {}
impl<C> Clone for State<C> where C: Clock
{
  fn clone(&self) -> Self {
    match self {
      | Self::Just(t) => Self::Just(*t),
      | Self::ConPreAck { timer,
                          post_ack_strategy,
                          post_ack_max_attempts, } => {
        Self::ConPreAck { timer: *timer,
                          post_ack_strategy: *post_ack_strategy,
                          post_ack_max_attempts: *post_ack_max_attempts }
      },
    }
  }
}

impl<C> Default for State<C> where C: Clock
{
  fn default() -> Self {
    Self::new(Instant::new(0),
              Strategy::Delay { min: Milliseconds(0),
                                max: Milliseconds(0) },
              Attempts::default())
  }
}

impl<C> State<C> where C: Clock
{
  fn new(time: Instant<C>, strat: Strategy, max_attempts: Attempts) -> Self {
    Self::Just(RetryTimer::new(time, strat, max_attempts))
  }

  fn timer(&mut self) -> &mut RetryTimer<C> {
    match self {
      | Self::Just(t) => t,
      | Self::ConPreAck { timer, .. } => timer,
    }
  }
}

/// Step that manages retrying outbound messages.
///
/// See the [module documentation](crate::step::retry) for more.
#[derive(Debug)]
pub struct Retry<Inner, Buffer> {
  inner: Inner,
  buf: Stem<Buffer>,
}

impl<Inner, Buffer> Default for Retry<Inner, Buffer>
  where Inner: Default,
        Buffer: Default
{
  fn default() -> Self {
    Self { inner: Inner::default(),
           buf: Stem::<Buffer>::default() }
  }
}

/// Errors that can be encountered when retrying messages
#[derive(PartialEq, Eq, PartialOrd, Clone, Copy)]
pub enum Error<E> {
  /// The inner step failed.
  ///
  /// This variant's Debug representation is completely
  /// replaced by the inner type E's debug representation.
  Inner(E),
  /// Storing this response would exceed a hard capacity for the
  /// buffer of messages waiting for responses.
  ///
  /// Only applicable to [`Retry`] that uses `ArrayVec` or
  /// similar heapless backing structure.
  RetryBufferFull,
}

impl<E: core::fmt::Debug> core::fmt::Debug for Error<E> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    match self {
      | Self::RetryBufferFull => f.debug_struct("RetryBufferFull").finish(),
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

impl<P, E, Inner, Buffer> Step<P> for Retry<Inner, Buffer>
  where Buffer: Buf<P>,
        P: PlatformTypes,
        E: super::Error,
        Inner: Step<P, PollReq = Addrd<Req<P>>, PollResp = Addrd<Resp<P>>, Error = E>
{
  type PollReq = Addrd<Req<P>>;
  type PollResp = Addrd<Resp<P>>;
  type Error = Error<E>;
  type Inner = Inner;

  fn inner(&self) -> &Inner {
    &self.inner
  }

  fn poll_req(&self,
              snap: &Snapshot<P>,
              effects: &mut <P as PlatformTypes>::Effects)
              -> StepOutput<Self::PollReq, Self::Error> {
    // SERVER FLOW:
    //  * CON responses WILL     be retried
    //  * NON responses WILL NOT be retried
    //  * ACKs          WILL NOT be retried
    //  * RESET         WILL NOT be retried
    _try!(Result; self.buf.map_mut(|b| b.attempt_all::<Inner::Error>(snap.time, effects)));

    let req = self.inner
                  .poll_req(snap, effects)
                  .map(|r| r.map_err(|nb| nb.map(Error::Inner)));
    let req = _try!(Option<nb::Result>; req);
    _try!(Result; self.buf.map_mut(|b| b.maybe_seen_response::<Inner::Error>(snap.time, effects, req.as_ref().map(|r| r.as_ref()))));
    Some(Ok(req))
  }

  fn poll_resp(&self,
               snap: &Snapshot<P>,
               effects: &mut <P as PlatformTypes>::Effects,
               token: toad_msg::Token,
               addr: no_std_net::SocketAddr)
               -> StepOutput<Self::PollResp, Self::Error> {
    // CLIENT FLOW:
    //  * CON requests WILL     be retried
    //  * NON requests WILL     be retried
    //  * RESET        WILL NOT be retried
    _try!(Result; self.buf.map_mut(|b| b.attempt_all::<Inner::Error>(snap.time, effects)));

    let resp =
      self.inner
          .poll_resp(snap, effects, token, addr)
          .map(|r| r.map_err(|nb| nb.map(Error::Inner)));
    let resp = _try!(Option<nb::Result>; resp);
    _try!(Result; self.buf.map_mut(|b| b.maybe_seen_response::<Inner::Error>(snap.time, effects, resp.as_ref().map(|r| r.as_ref()))));
    Some(Ok(resp))
  }

  fn on_message_sent(&self,
                     snap: &platform::Snapshot<P>,
                     effects: &mut P::Effects,
                     msg: &Addrd<platform::Message<P>>)
                     -> Result<(), Self::Error> {
    self.inner.on_message_sent(snap, effects, msg)?;
    self.buf
        .map_mut(|b| b.store_retryables(snap.time, effects, msg, snap.config))
  }
}

#[cfg(test)]
mod tests {
  use embedded_time::duration::Milliseconds;
  use tinyvec::array_vec;
  use toad_msg::{Code, Type};

  use super::*;
  use crate::config::{self, Config};
  use crate::platform::Effect;
  use crate::retry::Strategy;
  use crate::step::test::test_step;
  use crate::test::{self, ClockMock, Platform as P};

  type Retry<S> = super::Retry<S, Vec<(State<ClockMock>, Addrd<platform::Message<P>>)>>;

  fn snap_time(config: Config, time: u64) -> test::Snapshot {
    test::Snapshot { config,
                     recvd_dgram: Some(Addrd(tinyvec::array_vec!(1), test::dummy_addr())),
                     time: ClockMock::instant(time * 1000) }
  }

  fn config(con_delay: u64, sec_delay: u64) -> Config {
    let con_delay = Milliseconds(con_delay);
    let sec_delay = Milliseconds(sec_delay);
    let strategy_acked_con_or_non = Strategy::Delay { min: sec_delay,
                                                      max: sec_delay };
    Config { msg: config::Msg { con: config::Con { unacked_retry_strategy:
                                                     Strategy::Delay { min: con_delay,
                                                                       max: con_delay },
                                                   acked_retry_strategy:
                                                     strategy_acked_con_or_non,
                                                   ..Default::default() },
                                non: config::Non { retry_strategy:
                                                     strategy_acked_con_or_non,
                                                   ..Default::default() },
                                ..Default::default() },
             ..Default::default() }
  }

  type InnerPollReq = Addrd<test::Req>;
  type InnerPollResp = Addrd<test::Resp>;

  test_step!(
    GIVEN Retry::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
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
    GIVEN Retry::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
    WHEN inner_blocks [
      (inner.poll_req => { Some(Err(nb::Error::WouldBlock)) }),
      (inner.poll_resp => { Some(Err(nb::Error::WouldBlock)) })
    ]
    THEN this_should_block [
      (poll_req(_, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::WouldBlock))) }),
      (poll_resp(_, _, _, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::WouldBlock))) })
    ]
  );

  /*
   * | t      | what                                              |
   * | ------ | ------------------------------------------------- |
   * |     50 | CON request sent                                  |
   * |    250 | con_retry_strategy delay has passed, so we resend |
   * |    350 | got ACK, will use secondary_retry_strategy now    |
   * |    750 | sec_retry_strategy delay has passed, resend       |
   * |    850 | got response, will never retry again              |
   * | 10_000 | should not have retried                           |
   */
  #[test]
  fn when_con_request_sent_retry_should_retry_appropriately() {
    type Mock = test::MockStep<(), Addrd<test::Req>, Addrd<test::Resp>, ()>;
    let s = Retry::<Mock>::default();
    let token = Token(array_vec![1, 2, 3]);
    let token: &'static Token = unsafe { core::mem::transmute::<_, _>(&token) };
    s.inner()
     .set_poll_resp(|_, Snapshot { time, .. }, _, _, _| {
       let time: u64 = Milliseconds::try_from(time.duration_since_epoch()).unwrap()
                                                                          .0;

       let mut rep = test::msg!(ACK EMPTY x.x.x.x:0000);
       rep.as_mut().token = *token;

       match time {
         | 350 => Some(Ok(rep.map(Resp::from))),
         | 850 => {
           rep.as_mut().ty = Type::Non;
           rep.as_mut().code = Code::new(2, 4);
           Some(Ok(rep.map(Resp::from)))
         },
         | _ => None,
       }
     });
    let cfg = config(200, 400);
    let mut effs = Vec::<test::Effect>::new();
    macro_rules! sent {
       () => {{
         effs.iter().filter(|e| matches!(e, Effect::Log(_, _))).for_each(|e| match e {
           Effect::Log(l, m) => println!("[{:?}] {}", l, m.as_str()),
           _ => (),
         });
         effs.iter().filter(|e| matches!(e, Effect::Send(_))).collect::<Vec<&test::Effect>>()
       }};
     }

    let mut req = test::msg!(CON GET x.x.x.x:1111);
    req.as_mut().token = *token;

    s.on_message_sent(&snap_time(cfg, 50), &mut effs, &req)
     .unwrap();

    s.poll_resp(&snap_time(cfg, 150),
                &mut effs,
                req.data().token,
                req.addr())
     .ok_or(())
     .unwrap_err();
    assert_eq!(sent!().len(), 0);

    s.poll_resp(&snap_time(cfg, 250),
                &mut effs,
                req.data().token,
                req.addr())
     .ok_or(())
     .unwrap_err();
    assert_eq!(sent!().len(), 1);

    let ack = s.poll_resp(&snap_time(cfg, 350),
                          &mut effs,
                          req.data().token,
                          req.addr())
               .unwrap()
               .unwrap();
    assert_eq!(ack.data().msg().ty, Type::Ack);
    assert_eq!(sent!().len(), 1);

    s.poll_resp(&snap_time(cfg, 550),
                &mut effs,
                req.data().token,
                req.addr())
     .ok_or(())
     .unwrap_err();
    assert_eq!(sent!().len(), 1);

    s.poll_resp(&snap_time(cfg, 750),
                &mut effs,
                req.data().token,
                req.addr())
     .ok_or(())
     .unwrap_err();
    assert_eq!(sent!().len(), 2);

    let rep = s.poll_resp(&snap_time(cfg, 850),
                          &mut effs,
                          req.data().token,
                          req.addr())
               .unwrap()
               .unwrap();
    assert_eq!(rep.data().msg().ty, Type::Non);
    assert_eq!(sent!().len(), 2);

    s.poll_resp(&snap_time(cfg, 10_000),
                &mut effs,
                req.data().token,
                req.addr())
     .ok_or(())
     .unwrap_err();
    assert_eq!(sent!().len(), 2);
  }

  /*
   * | t      | what                                              |
   * | ------ | ------------------------------------------------- |
   * |     50 | CON request sent                                  |
   * |    100 | got RESET                                         |
   * | 10_000 | should not have retried                           |
   */
  #[test]
  fn when_outbound_message_reset_retry_should_not_retry() {
    type Mock = test::MockStep<(), Addrd<test::Req>, Addrd<test::Resp>, ()>;
    let s = Retry::<Mock>::default();

    let token_a = Token(array_vec![1, 2, 3]);
    let token_a: &'static Token = unsafe { core::mem::transmute::<_, _>(&token_a) };

    let token_b = Token(array_vec![1, 2, 4]);
    let token_b: &'static Token = unsafe { core::mem::transmute::<_, _>(&token_b) };

    let token_c = Token(array_vec![1, 2, 5]);
    let token_c: &'static Token = unsafe { core::mem::transmute::<_, _>(&token_c) };

    s.inner()
     .set_poll_resp(|_, Snapshot { time, .. }, _, token, _| {
       let time: u64 = Milliseconds::try_from(time.duration_since_epoch()).unwrap()
                                                                          .0;

       let mut rst = test::msg!(RESET x.x.x.x:0000);
       rst.as_mut().token = token;

       match time {
         | 150 => Some(Ok(rst.map(Resp::from))),
         | _ => None,
       }
     })
     .set_poll_req(|_, Snapshot { time, .. }, _| {
       let time: u64 = Milliseconds::try_from(time.duration_since_epoch()).unwrap()
                                                                          .0;

       let mut rst = test::msg!(RESET x.x.x.x:0000);
       rst.as_mut().token = *token_c;

       match time {
         | 150 => Some(Ok(rst.map(Req::from))),
         | _ => None,
       }
     });
    let cfg = config(200, 400);
    let mut effs = Vec::<test::Effect>::new();
    macro_rules! sent {
       () => {{
         effs.iter().filter(|e| matches!(e, Effect::Log(_, _))).for_each(|e| match e {
           Effect::Log(l, m) => println!("[{:?}] {}", l, m.as_str()),
           _ => (),
         });
         effs.iter().filter(|e| matches!(e, Effect::Send(_))).collect::<Vec<&test::Effect>>()
       }};
     }

    let mut con_req = test::msg!(CON GET x.x.x.x:1111);
    con_req.as_mut().token = *token_a;

    let mut non_req = test::msg!(NON GET x.x.x.x:1111);
    non_req.as_mut().token = *token_b;

    let mut con_rep = test::msg!(CON {2 . 04} x.x.x.x:1111);
    con_rep.as_mut().token = *token_c;

    s.on_message_sent(&snap_time(cfg, 50), &mut effs, &con_rep)
     .unwrap();
    s.on_message_sent(&snap_time(cfg, 50), &mut effs, &con_req)
     .unwrap();
    s.on_message_sent(&snap_time(cfg, 50), &mut effs, &non_req)
     .unwrap();

    let rep = s.poll_resp(&snap_time(cfg, 150), &mut effs, *token_a, con_req.addr())
               .unwrap()
               .unwrap();
    assert_eq!(sent!().len(), 0);
    assert_eq!(rep.data().msg().ty, Type::Reset);

    let rep = s.poll_resp(&snap_time(cfg, 150), &mut effs, *token_b, con_req.addr())
               .unwrap()
               .unwrap();
    assert_eq!(sent!().len(), 0);
    assert_eq!(rep.data().msg().ty, Type::Reset);

    let req = s.poll_req(&snap_time(cfg, 150), &mut effs)
               .unwrap()
               .unwrap();
    assert_eq!(sent!().len(), 0);
    assert_eq!(req.data().msg().ty, Type::Reset);

    s.poll_resp(&snap_time(cfg, 10_000), &mut effs, *token_a, con_req.addr())
     .ok_or(())
     .unwrap_err();
    s.poll_resp(&snap_time(cfg, 10_000), &mut effs, *token_b, con_req.addr())
     .ok_or(())
     .unwrap_err();
    s.poll_req(&snap_time(cfg, 10_000), &mut effs)
     .ok_or(())
     .unwrap_err();

    assert_eq!(sent!().len(), 0);
  }

  /*
   * | t      | what                                              |
   * | ------ | ------------------------------------------------- |
   * |     50 | CON response sent                                 |
   * |    250 | con_retry_strategy delay has passed, so we resend |
   * |    350 | got ACK, will never retry again                   |
   * |    750 | sec_retry_strategy delay has passed, no resend    |
   * | 10_000 | should not have retried                           |
   */
  #[test]
  fn when_con_response_sent_retry_should_retry_appropriately() {
    type Mock = test::MockStep<(), Addrd<test::Req>, Addrd<test::Resp>, ()>;
    let s = Retry::<Mock>::default();
    let token = Token(array_vec![1, 2, 3]);
    let token: &'static Token = unsafe { core::mem::transmute::<_, _>(&token) };
    s.inner().set_poll_req(|_, Snapshot { time, .. }, _| {
               let time: u64 = Milliseconds::try_from(time.duration_since_epoch()).unwrap()
                                                                                  .0;

               let mut ack = test::msg!(ACK EMPTY x.x.x.x:0000);
               ack.as_mut().token = *token;

               match time {
                 | 350 => Some(Ok(ack.map(Req::from))),
                 | _ => None,
               }
             });
    let cfg = config(200, 400);
    let mut effs = Vec::<test::Effect>::new();
    macro_rules! sent {
       () => {{
         effs.iter().filter(|e| matches!(e, Effect::Log(_, _))).for_each(|e| match e {
           Effect::Log(l, m) => println!("[{:?}] {}", l, m.as_str()),
           _ => (),
         });
         effs.iter().filter(|e| matches!(e, Effect::Send(_))).collect::<Vec<&test::Effect>>()
       }};
     }

    let mut rep = test::msg!(CON {2 . 04} x.x.x.x:1111);
    rep.as_mut().token = *token;

    s.on_message_sent(&snap_time(cfg, 50), &mut effs, &rep)
     .unwrap();

    s.poll_req(&snap_time(cfg, 150), &mut effs)
     .ok_or(())
     .unwrap_err();
    assert_eq!(sent!().len(), 0);

    s.poll_req(&snap_time(cfg, 250), &mut effs)
     .ok_or(())
     .unwrap_err();
    assert_eq!(sent!().len(), 1);

    let ack = s.poll_req(&snap_time(cfg, 350), &mut effs)
               .unwrap()
               .unwrap();
    assert_eq!(ack.data().msg().ty, Type::Ack);
    assert_eq!(sent!().len(), 1);

    s.poll_req(&snap_time(cfg, 10_000), &mut effs)
     .ok_or(())
     .unwrap_err();
    assert_eq!(sent!().len(), 1);
  }

  /*
   * | t      | what                                              |
   * | ------ | ------------------------------------------------- |
   * |     50 | NON request sent                                  |
   * |    250 | non_retry_strategy delay has passed, so we resend |
   * |    350 | got response, will never retry again              |
   * | 10_000 | should not have retried                           |
   */
  #[test]
  fn when_non_request_sent_retry_should_retry_appropriately() {
    type Mock = test::MockStep<(), Addrd<test::Req>, Addrd<test::Resp>, ()>;
    let s = Retry::<Mock>::default();
    let token = Token(array_vec![1, 2, 3]);
    let token: &'static Token = unsafe { core::mem::transmute::<_, _>(&token) };
    s.inner()
     .set_poll_resp(|_, Snapshot { time, .. }, _, _, _| {
       let time: u64 = Milliseconds::try_from(time.duration_since_epoch()).unwrap()
                                                                          .0;

       let mut rep = test::msg!(NON {2 . 04} x.x.x.x:0000);
       rep.as_mut().token = *token;

       match time {
         | 350 => Some(Ok(rep.map(Resp::from))),
         | _ => None,
       }
     });
    let cfg = config(200, 200);
    let mut effs = Vec::<test::Effect>::new();
    macro_rules! sent {
       () => {{
         effs.iter().filter(|e| matches!(e, Effect::Log(_, _))).for_each(|e| match e {
           Effect::Log(l, m) => println!("[{:?}] {}", l, m.as_str()),
           _ => (),
         });
         effs.iter().filter(|e| matches!(e, Effect::Send(_))).collect::<Vec<&test::Effect>>()
       }};
     }

    let mut req = test::msg!(NON GET x.x.x.x:1111);
    req.as_mut().token = *token;

    s.on_message_sent(&snap_time(cfg, 50), &mut effs, &req)
     .unwrap();

    s.poll_resp(&snap_time(cfg, 150),
                &mut effs,
                req.data().token,
                req.addr())
     .ok_or(())
     .unwrap_err();
    assert_eq!(sent!().len(), 0);

    s.poll_resp(&snap_time(cfg, 250),
                &mut effs,
                req.data().token,
                req.addr())
     .ok_or(())
     .unwrap_err();
    assert_eq!(sent!().len(), 1);

    let rep = s.poll_resp(&snap_time(cfg, 350),
                          &mut effs,
                          req.data().token,
                          req.addr())
               .unwrap()
               .unwrap();
    assert_eq!(rep.data().msg().ty, Type::Non);
    assert_eq!(sent!().len(), 1);

    s.poll_resp(&snap_time(cfg, 10_000),
                &mut effs,
                req.data().token,
                req.addr())
     .ok_or(())
     .unwrap_err();
    assert_eq!(sent!().len(), 1);
  }

  /*
   * | t      | what                                              |
   * | ------ | ------------------------------------------------- |
   * |     50 | ACK response sent                                 |
   * | 10_000 | should not have retried                           |
   */
  #[test]
  fn when_ack_response_sent_retry_should_never_retry() {
    type Mock = test::MockStep<(), Addrd<test::Req>, Addrd<test::Resp>, ()>;
    let s = Retry::<Mock>::default();
    let token = Token(array_vec![1, 2, 3]);
    let token: &'static Token = unsafe { core::mem::transmute::<_, _>(&token) };

    let cfg = config(200, 200);
    let mut effs = Vec::<test::Effect>::new();
    macro_rules! sent {
       () => {{
         effs.iter().filter(|e| matches!(e, Effect::Log(_, _))).for_each(|e| match e {
           Effect::Log(l, m) => println!("[{:?}] {}", l, m.as_str()),
           _ => (),
         });
         effs.iter().filter(|e| matches!(e, Effect::Send(_))).collect::<Vec<&test::Effect>>()
       }};
     }

    let mut ack = test::msg!(ACK EMPTY x.x.x.x:1111);
    ack.as_mut().token = *token;

    s.on_message_sent(&snap_time(cfg, 50), &mut effs, &ack)
     .unwrap();

    s.poll_resp(&snap_time(cfg, 10_000),
                &mut effs,
                ack.data().token,
                ack.addr())
     .ok_or(())
     .unwrap_err();
    assert_eq!(sent!().len(), 0);

    s.poll_req(&snap_time(cfg, 10_000), &mut effs)
     .ok_or(())
     .unwrap_err();
    assert_eq!(sent!().len(), 0);
  }

  /*
   * | t      | what                                              |
   * | ------ | ------------------------------------------------- |
   * |     50 | NON response sent                                 |
   * | 10_000 | should not have retried                           |
   */
  #[test]
  fn when_non_response_sent_retry_should_never_retry() {
    type Mock = test::MockStep<(), Addrd<test::Req>, Addrd<test::Resp>, ()>;
    let s = Retry::<Mock>::default();
    let token = Token(array_vec![1, 2, 3]);
    let token: &'static Token = unsafe { core::mem::transmute::<_, _>(&token) };

    let cfg = config(200, 200);
    let mut effs = Vec::<test::Effect>::new();
    macro_rules! sent {
       () => {{
         effs.iter().filter(|e| matches!(e, Effect::Log(_, _))).for_each(|e| match e {
           Effect::Log(l, m) => println!("[{:?}] {}", l, m.as_str()),
           _ => (),
         });
         effs.iter().filter(|e| matches!(e, Effect::Send(_))).collect::<Vec<&test::Effect>>()
       }};
     }

    let mut rep = test::msg!(NON {2 . 04} x.x.x.x:1111);
    rep.as_mut().token = *token;

    s.on_message_sent(&snap_time(cfg, 50), &mut effs, &rep)
     .unwrap();

    s.poll_resp(&snap_time(cfg, 10_000),
                &mut effs,
                rep.data().token,
                rep.addr())
     .ok_or(())
     .unwrap_err();
    assert_eq!(sent!().len(), 0);

    s.poll_req(&snap_time(cfg, 10_000), &mut effs)
     .ok_or(())
     .unwrap_err();
    assert_eq!(sent!().len(), 0);
  }

  /*
   * | t      | what                                              |
   * | ------ | ------------------------------------------------- |
   * |     50 | RESET response sent                               |
   * | 10_000 | should not have retried                           |
   */
  #[test]
  fn when_reset_sent_retry_should_never_retry() {
    type Mock = test::MockStep<(), Addrd<test::Req>, Addrd<test::Resp>, ()>;
    let s = Retry::<Mock>::default();
    let token = Token(array_vec![1, 2, 3]);
    let token: &'static Token = unsafe { core::mem::transmute::<_, _>(&token) };

    let cfg = config(200, 200);
    let mut effs = Vec::<test::Effect>::new();
    macro_rules! sent {
       () => {{
         effs.iter().filter(|e| matches!(e, Effect::Log(_, _))).for_each(|e| match e {
           Effect::Log(l, m) => println!("[{:?}] {}", l, m.as_str()),
           _ => (),
         });
         effs.iter().filter(|e| matches!(e, Effect::Send(_))).collect::<Vec<&test::Effect>>()
       }};
     }

    let mut rst = test::msg!(RESET x.x.x.x:1111);
    rst.as_mut().token = *token;

    s.on_message_sent(&snap_time(cfg, 50), &mut effs, &rst)
     .unwrap();

    s.poll_resp(&snap_time(cfg, 10_000),
                &mut effs,
                rst.data().token,
                rst.addr())
     .ok_or(())
     .unwrap_err();
    assert_eq!(sent!().len(), 0);

    s.poll_req(&snap_time(cfg, 10_000), &mut effs)
     .ok_or(())
     .unwrap_err();
    assert_eq!(sent!().len(), 0);
  }
}
