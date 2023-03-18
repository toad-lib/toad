use embedded_time::duration::Milliseconds;
use embedded_time::Instant;
use toad_array::Array;
use toad_msg::{CodeKind, Token, Type};
use toad_stem::Stem;

use super::{Step, StepOutput, _try};
use crate::config::Config;
use crate::net::Addrd;
use crate::platform::{self, Effect, PlatformTypes, Snapshot};
use crate::req::Req;
use crate::resp::Resp;
use crate::retry::{Attempts, RetryTimer, Strategy, YouShould};
use crate::time::Clock;

/// Buffer used to store messages queued for retry
pub trait Buf<P>
  where P: PlatformTypes,
        Self: Array<Item = (State<P::Clock>, Addrd<platform::Message<P>>)>
{
  /// Do some black box magic to send all messages that need to be sent
  fn attempt_all<E>(&mut self,
                    time: Instant<P::Clock>,
                    effects: &mut <P as PlatformTypes>::Effects)
                    -> Result<(), Error<E>> {
    self.iter_mut()
        .filter_map(|(state, msg)| match state.timer().what_should_i_do(time) {
          | Ok(YouShould::Retry) => Some((state, msg)),
          | _ => None,
        })
        .try_for_each(|(_, msg)| -> Result<(), Error<E>> {
          effects.push(Effect::Send(msg.clone()));
          Ok(())
        })
  }

  /// We saw a response and should remove all tracking of a token (if we have any)
  fn forget(&mut self, token: Token) {
    match self.iter()
              .enumerate()
              .find(|(_, (_, msg))| msg.data().token == token)
    {
      | Some((ix, _)) => {
        self.remove(ix);
      },
      | _ => (),
    }
  }

  /// We saw an ACK and should transition the retry state for matching outbound
  /// CONs to the "acked" state
  fn mark_acked(&mut self, token: Token, time: Instant<P::Clock>) {
    let found = self.iter()
                    .enumerate()
                    .find(|(_, (_, msg))| msg.data().token == token);

    let (ix, new_timer) = match found {
      | Some((ix, _)) if self[ix].1.data().code.kind() == CodeKind::Response => {
        return self.forget(token)
      },
      | Some((ix,
              (State::ConPreAck { post_ack_strategy,
                                  post_ack_max_attempts,
                                  .. },
               _))) => (ix, RetryTimer::new(time, *post_ack_strategy, *post_ack_max_attempts)),
      | _ => return,
    };

    self.get_mut(ix).unwrap().0 = State::Just(new_timer);
  }

  /// Called when a response of any kind to any request is
  /// received
  ///
  /// May invoke `mark_acked` & `forget`
  fn maybe_seen_response<E>(&mut self,
                            time: Instant<P::Clock>,
                            msg: Addrd<&platform::Message<P>>)
                            -> Result<(), Error<E>> {
    match (msg.data().ty, msg.data().code.kind()) {
      | (Type::Ack, CodeKind::Empty) => {
        self.mark_acked(msg.data().token, time);
        Ok(())
      },
      | (_, CodeKind::Response) => {
        self.forget(msg.data().token);
        Ok(())
      },
      | _ => Ok(()),
    }
  }

  /// Called when a message of any kind is sent,
  /// and may store it to be retried in the future
  fn store_retryables<E>(&mut self,
                         msg: &Addrd<platform::Message<P>>,
                         time: Instant<P::Clock>,
                         config: Config)
                         -> Result<(), Error<E>> {
    match msg.data().ty {
      | Type::Con | Type::Non if self.is_full() => Err(Error::RetryBufferFull),
      | Type::Con => {
        self.push((State::ConPreAck { timer: RetryTimer::new(time,
                                                             config.msg
                                                                   .con
                                                                   .unacked_retry_strategy,
                                                             config.msg.con.max_attempts),
                                      post_ack_strategy: config.msg.con.acked_retry_strategy,
                                      post_ack_max_attempts: config.msg.con.max_attempts },
                   msg.clone()));

        Ok(())
      },
      | Type::Non if msg.data().code.kind() == CodeKind::Request => {
        self.push((State::Just(RetryTimer::new(time,
                                               config.msg.non.retry_strategy,
                                               config.msg.non.max_attempts)),
                   msg.clone()));

        Ok(())
      },
      | _ => Ok(()),
    }
  }
}

impl<T, P> Buf<P> for T
  where T: Array<Item = (State<P::Clock>, Addrd<platform::Message<P>>)>,
        P: PlatformTypes
{
}

/// The state of a message stored in the retry [buffer](Buf)
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
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
    _try!(Result; self.buf.map_mut(|b| b.maybe_seen_response::<Inner::Error>(snap.time, req.as_ref().map(|r| r.as_ref()))));
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
    _try!(Result; self.buf.map_mut(|b| b.maybe_seen_response::<Inner::Error>(snap.time, resp.as_ref().map(|r| r.as_ref()))));
    Some(Ok(resp))
  }

  fn on_message_sent(&self,
                     snap: &platform::Snapshot<P>,
                     msg: &Addrd<platform::Message<P>>)
                     -> Result<(), Self::Error> {
    self.inner.on_message_sent(snap, msg)?;
    self.buf
        .map_mut(|b| b.store_retryables(msg, snap.time, snap.config))
  }
}

#[cfg(test)]
mod tests {
  use embedded_time::duration::Milliseconds;
  use toad_msg::Type;

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
  test_step!(
    GIVEN Retry::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
    WHEN con_request_sent [
      (inner.poll_resp = {
        |Snapshot {time, ..}, _, _, _| {
          let time: u64 = Milliseconds::try_from(time.duration_since_epoch()).unwrap().0;

          match time {
            350 => Some(Ok(test::msg!(ACK EMPTY x.x.x.x:0000).map(Resp::from))),
            850 => Some(Ok(test::msg!(NON {2 . 04} x.x.x.x:0000).map(Resp::from))),
            _ => None,
          }
        }
      })
    ]
    THEN this_should_retry_appropriately /* see comment above */ [
      (
        on_message_sent(
          snap_time(config(200, 400), 50),
          test::msg!(CON GET x.x.x.x:1111)
        ) should satisfy { |_| () }
      ),
      (poll_resp(snap_time(config(200, 400), 150), _, _, _) should satisfy { |out| assert_eq!(out, None) }),
      (effects should satisfy { |e| assert_eq!(e, &vec![]) }),
      (poll_resp(snap_time(config(200, 400), 250), _, _, _) should satisfy { |out| assert_eq!(out, None) }),
      (effects should satisfy {
        |e| {
          assert_eq!(e, &vec![Effect::Send(test::msg!(CON GET x.x.x.x:1111))])
        }
      }),
      (
        poll_resp(
          snap_time(config(200, 400), 350),
          _,
          _,
          _
        ) should satisfy {
          |out| assert!(matches!(out, Some(Ok(r)) if r.data().as_ref().ty == Type::Ack))
        }
      ),
      (poll_resp(snap_time(config(200, 400), 550), _, _, _) should satisfy { |out| assert_eq!(out, None) }),
      (effects should satisfy {
        |e| {
          assert_eq!(e, &vec![Effect::Send(test::msg!(CON GET x.x.x.x:1111))])
        }
      }),
      (poll_resp(snap_time(config(200, 400), 750), _, _, _) should satisfy { |out| assert_eq!(out, None) }),
      (effects should satisfy {
        |e| {
          assert_eq!(e, &vec![Effect::Send(test::msg!(CON GET x.x.x.x:1111)), Effect::Send(test::msg!(CON GET x.x.x.x:1111))])
        }
      }),
      (
        poll_resp(
          snap_time(config(200, 400), 850),
          _,
          _,
          _
        ) should satisfy {
          |out| assert!(matches!(out, Some(Ok(rep)) if rep.data().as_ref().ty == Type::Non))
        }
      ),
      (poll_resp(snap_time(config(200, 400), 10_000), _, _, _) should satisfy { |out| assert_eq!(out, None) }),
      (effects should satisfy {
        |e| {
          assert_eq!(e, &vec![Effect::Send(test::msg!(CON GET x.x.x.x:1111)), Effect::Send(test::msg!(CON GET x.x.x.x:1111))])
        }
      })
    ]
  );

  /*
   * | t      | what                                              |
   * | ------ | ------------------------------------------------- |
   * |     50 | CON response sent                                 |
   * |    250 | con_retry_strategy delay has passed, so we resend |
   * |    350 | got ACK, will never retry again                   |
   * |    750 | sec_retry_strategy delay has passed, no resend    |
   * | 10_000 | should not have retried                           |
   */
  test_step!(
    GIVEN Retry::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
    WHEN con_response_sent [
      (inner.poll_req = {
        |Snapshot {time, ..}, _| {
          let time: u64 = Milliseconds::try_from(time.duration_since_epoch()).unwrap().0;

          match time {
            350 => Some(Ok(test::msg!(ACK EMPTY x.x.x.x:0000).map(Req::from))),
            _ => None
          }
        }
      })
    ]
    THEN this_should_retry_appropriately /* see comment above */ [
      (
        on_message_sent(
          snap_time(config(200, 400), 50),
          test::msg!(CON {2 . 04} x.x.x.x:1111)
        ) should satisfy { |_| () }
      ),
      (poll_req(snap_time(config(200, 400), 150), _) should satisfy { |out| assert_eq!(out, None) }),
      (effects should satisfy { |e| assert_eq!(e, &vec![]) }),
      (poll_req(snap_time(config(200, 400), 250), _) should satisfy { |out| assert_eq!(out, None) }),
      (effects should satisfy {
        |e| {
          assert_eq!(e, &vec![Effect::Send(test::msg!(CON {2 . 04} x.x.x.x:1111))])
        }
      }),
      (
        poll_req(
          snap_time(config(200, 400), 350),
          _
        ) should satisfy {
          |out| assert!(matches!(out, Some(Ok(r)) if r.data().as_ref().ty == Type::Ack))
        }
      ),
      (poll_req(snap_time(config(200, 400), 550), _) should satisfy { |out| assert_eq!(out, None) }),
      (effects should satisfy {
        |e| {
          assert_eq!(e, &vec![Effect::Send(test::msg!(CON {2 . 04} x.x.x.x:1111))])
        }
      }),
      (poll_req(snap_time(config(200, 400), 750), _) should satisfy { |out| assert_eq!(out, None) }),
      (effects should satisfy {
        |e| {
          assert_eq!(e, &vec![Effect::Send(test::msg!(CON {2 . 04} x.x.x.x:1111))])
        }
      }),
      (poll_req(snap_time(config(200, 400), 10_000), _) should satisfy { |out| assert_eq!(out, None) }),
      (effects should satisfy {
        |e| {
          assert_eq!(e, &vec![Effect::Send(test::msg!(CON {2 . 04} x.x.x.x:1111))])
        }
      })
    ]
  );

  /*
   * | t      | what                                              |
   * | ------ | ------------------------------------------------- |
   * |     50 | NON request sent                                  |
   * |    250 | non_retry_strategy delay has passed, so we resend |
   * |    350 | got response, will never retry again              |
   * | 10_000 | should not have retried                           |
   */
  test_step!(
    GIVEN Retry::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
    WHEN non_request_sent [
      (inner.poll_resp = {
        |Snapshot {time, ..}, _, _, _| {
          let time: u64 = Milliseconds::try_from(time.duration_since_epoch()).unwrap().0;

          match time {
            350 => Some(Ok(test::msg!(NON {2 . 04} x.x.x.x:0000).map(Resp::from))),
                _ => None,
          }
        }
      })
    ]
    THEN this_should_retry_appropriately /* see comment above */ [
      (
        on_message_sent(
          snap_time(config(200, 200), 50),
          test::msg!(NON GET x.x.x.x:1111)
        ) should satisfy { |_| () }
      ),
      (poll_resp(snap_time(config(200, 200), 150), _, _, _) should satisfy { |out| assert_eq!(out, None) }),
      (effects should satisfy { |e| assert_eq!(e, &vec![]) }),
      (poll_resp(snap_time(config(200, 200), 250), _, _, _) should satisfy { |out| assert_eq!(out, None) }),
      (effects should satisfy {
        |e| {
          assert_eq!(e, &vec![Effect::Send(test::msg!(NON GET x.x.x.x:1111))])
        }
      }),
      (
        poll_resp(
          snap_time(config(200, 200), 350),
          _,
          _,
          _
        ) should satisfy {
          |out| assert!(matches!(out, Some(Ok(r)) if r.data().as_ref().ty == Type::Non))
        }
      ),
      (effects should satisfy {
        |e| {
          assert_eq!(e, &vec![Effect::Send(test::msg!(NON GET x.x.x.x:1111))])
        }
      }),
      (poll_resp(snap_time(config(200, 200), 10_000), _, _, _) should satisfy { |out| assert_eq!(out, None) }),
      (effects should satisfy {
        |e| {
          assert_eq!(e, &vec![Effect::Send(test::msg!(NON GET x.x.x.x:1111))])
        }
      })
    ]
  );

  /*
   * | t      | what                                              |
   * | ------ | ------------------------------------------------- |
   * |     50 | ACK response sent                                 |
   * |    250 | con_retry_strategy delay has passed               |
   * |    --- | but no resend should occur                        |
   * | 10_000 | should not have retried                           |
   */
  test_step!(
    GIVEN Retry::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
    WHEN we_send_ack [
      (inner.poll_req => {None})
    ]
    THEN this_should_never_retry /* see comment above */ [
      (
        on_message_sent(
          snap_time(config(200, 400), 50),
          test::msg!(ACK EMPTY x.x.x.x:0000)
        ) should satisfy { |_| () }
      ),
      (poll_resp(snap_time(config(200, 400), 150), _, _, _) should satisfy { |out| assert_eq!(out, None) }),
      (effects should satisfy { |e| assert_eq!(e, &vec![]) }),
      (poll_resp(snap_time(config(200, 400), 250), _, _, _) should satisfy { |out| assert_eq!(out, None) }),
      (effects should satisfy { |e| assert_eq!(e, &vec![]) }),
      (poll_resp(snap_time(config(200, 400), 10_000), _, _, _) should satisfy { |out| assert_eq!(out, None) }),
      (effects should satisfy { |e| assert_eq!(e, &vec![]) })
    ]
  );

  /*
   * | t      | what                                              |
   * | ------ | ------------------------------------------------- |
   * |     50 | NON response sent                                 |
   * |    250 | con_retry_strategy delay has passed               |
   * |    --- | but no resend should occur                        |
   * | 10_000 | should not have retried                           |
   */
  test_step!(
    GIVEN Retry::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
    WHEN we_send_non_response [
      (inner.poll_req => {None})
    ]
    THEN this_should_never_retry /* see comment above */ [
      (
        on_message_sent(
          snap_time(config(200, 400), 50),
          test::msg!(NON {2 . 05} x.x.x.x:0000)
        ) should satisfy { |_| () }
      ),
      (poll_resp(snap_time(config(200, 400), 150), _, _, _) should satisfy { |out| assert_eq!(out, None) }),
      (effects should satisfy { |e| assert_eq!(e, &vec![]) }),
      (poll_resp(snap_time(config(200, 400), 250), _, _, _) should satisfy { |out| assert_eq!(out, None) }),
      (effects should satisfy { |e| assert_eq!(e, &vec![]) }),
      (poll_resp(snap_time(config(200, 400), 10_000), _, _, _) should satisfy { |out| assert_eq!(out, None) }),
      (effects should satisfy { |e| assert_eq!(e, &vec![]) })
    ]
  );

  /*
   * | t      | what                                              |
   * | ------ | ------------------------------------------------- |
   * |     50 | RESET response sent                               |
   * |    250 | con_retry_strategy delay has passed               |
   * |    --- | but no resend should occur                        |
   * | 10_000 | should not have retried                           |
   */
  test_step!(
    GIVEN Retry::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
    WHEN we_send_reset_request_or_response [
      (inner.poll_req => {None}),
      (inner.poll_resp => {None})
    ]
    THEN this_should_never_retry [
      (
        on_message_sent(
          snap_time(config(200, 400), 50),
          test::msg!(NON {2 . 05} x.x.x.x:0000)
        ) should satisfy { |_| () }
      ),
      (poll_resp(snap_time(config(200, 400), 150), _, _, _) should satisfy { |out| assert_eq!(out, None) }),
      (effects should satisfy { |e| assert_eq!(e, &vec![]) }),
      (poll_resp(snap_time(config(200, 400), 250), _, _, _) should satisfy { |out| assert_eq!(out, None) }),
      (effects should satisfy { |e| assert_eq!(e, &vec![]) }),
      (poll_resp(snap_time(config(200, 400), 10_000), _, _, _) should satisfy { |out| assert_eq!(out, None) }),
      (effects should satisfy { |e| assert_eq!(e, &vec![]) }),
      (poll_req(snap_time(config(200, 400), 150), _) should satisfy { |out| assert_eq!(out, None) }),
      (effects should satisfy { |e| assert_eq!(e, &vec![]) }),
      (poll_req(snap_time(config(200, 400), 250), _) should satisfy { |out| assert_eq!(out, None) }),
      (effects should satisfy { |e| assert_eq!(e, &vec![]) }),
      (poll_req(snap_time(config(200, 400), 10_000), _) should satisfy { |out| assert_eq!(out, None) }),
      (effects should satisfy { |e| assert_eq!(e, &vec![]) })
    ]
  );
}
