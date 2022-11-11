use embedded_time::duration::Milliseconds;
use embedded_time::Instant;
use toad_common::{Array, GetSize};
use toad_msg::to_bytes::MessageToBytesError;
use toad_msg::{CodeKind, Token, TryIntoBytes, Type};

use super::{Step, StepOutput, _try};
use crate::config::Config;
use crate::net::Addrd;
use crate::platform::{self, Effect, Platform, Snapshot};
use crate::req::Req;
use crate::resp::Resp;
use crate::retry::{Attempts, RetryTimer, Strategy, YouShould};
use crate::time::Clock;

/// TODO
pub trait Buf<P>
  where P: Platform,
        Self: Array<Item = (State<P::Clock>, Addrd<platform::Message<P>>)>
{
  /// TODO
  fn attempt_all<E>(&mut self,
                    time: Instant<P::Clock>,
                    effects: &mut <P as Platform>::Effects)
                    -> Result<(), Error<E>> {
    self.iter_mut()
        .filter_map(|(state, msg)| match state.timer().what_should_i_do(time) {
          | Ok(YouShould::Retry) => Some((state, msg)),
          | _ => None,
        })
        .try_for_each(|(_, msg)| -> Result<(), Error<E>> {
          let bytes = msg.data()
                         .clone() // TODO: remove this clone when
                         // try_from_bytes is `&self -> Result<_, _>`,
                         // instead of `self -> Result<_, _>`
                         .try_into_bytes()
                         .map_err(Error::RetrySerializingFailed)?;
          effects.push(Effect::SendDgram(msg.as_ref().map(|_| bytes)));
          Ok(())
        })
  }

  /// TODO
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

  /// TODO
  fn mark_acked(&mut self, token: Token, time: Instant<P::Clock>) {
    let (ix, new_timer) = match self.iter()
                                    .enumerate()
                                    .find(|(_, (_, msg))| msg.data().token == token)
    {
      | Some((ix,
              (State::ConPreAck { post_ack_strategy,
                                  post_ack_max_attempts,
                                  .. },
               _))) => (ix, RetryTimer::new(time, *post_ack_strategy, *post_ack_max_attempts)),
      | _ => return,
    };

    self.get_mut(ix).unwrap().0 = State::Just(new_timer);
  }

  /// TODO
  fn seen_response<E>(&mut self,
                      time: Instant<P::Clock>,
                      msg: &Addrd<Resp<P>>)
                      -> Result<(), Error<E>> {
    match msg.data().msg.ty {
      | Type::Ack if msg.data().msg.code.kind() == CodeKind::Empty => {
        Ok(self.mark_acked(msg.data().msg.token, time))
      },
      | _ if msg.data().msg.code.kind() == CodeKind::Response => {
        Ok(self.forget(msg.data().msg.token))
      },
      | _ => Ok(()),
    }
  }

  /// TODO
  fn store_retryables<E>(&mut self,
                         msg: &Addrd<platform::Message<P>>,
                         time: Instant<P::Clock>,
                         config: Config)
                         -> Result<(), Error<E>> {
    macro_rules! con_preack_retry_timer {
      () => {
        RetryTimer::new(time,
                        config.msg.con_requests.unacked_retry_strategy,
                        config.msg.con_requests.max_attempts)
      };
    }

    macro_rules! non_retry_timer {
      () => {
        RetryTimer::new(time,
                        config.msg.non_requests.retry_strategy,
                        config.msg.non_requests.max_attempts)
      };
    }

    match msg.data().ty {
      | Type::Con | Type::Non if self.is_full() => Err(Error::RetryBufferFull),
      | Type::Con => {
        self.push((State::ConPreAck { timer: con_preack_retry_timer!(),
                                      post_ack_strategy: config.msg
                                                               .con_requests
                                                               .acked_retry_strategy,
                                      post_ack_max_attempts: config.msg
                                                                   .con_requests
                                                                   .max_attempts },
                   msg.clone()));

        Ok(())
      },
      | Type::Non if msg.data().code.kind() == CodeKind::Request => {
        self.push((State::Just(non_retry_timer!()), msg.clone()));

        Ok(())
      },
      | _ => Ok(()),
    }
  }
}

impl<T, P> Buf<P> for T
  where T: Array<Item = (State<P::Clock>, Addrd<platform::Message<P>>)>,
        P: Platform
{
}

/// TODO
#[cfg(feature = "alloc")]
pub mod alloc {
  use std_alloc::vec::Vec;

  use crate::net::Addrd;
  use crate::platform;

  /// TODO
  pub type Retry<P, Inner> = super::Retry<Inner,
                                          Vec<(super::State<<P as platform::Platform>::Clock>,
                                               Addrd<platform::Message<P>>)>>;
}

/// TODO
pub mod no_alloc {
  use tinyvec::ArrayVec;

  use crate::net::Addrd;
  use crate::platform;

  /// TODO
  pub type Retry<P, Inner, const N: usize = 16> =
    super::Retry<Inner,
                 ArrayVec<[(super::State<<P as platform::Platform>::Clock>,
                           Addrd<platform::Message<P>>); N]>>;
}

/// TODO
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum State<C>
  where C: Clock
{
  /// TODO
  Just(RetryTimer<C>),
  /// TODO
  ConPreAck {
    /// TODO
    timer: RetryTimer<C>,
    /// TODO
    post_ack_strategy: Strategy,
    /// TODO
    post_ack_max_attempts: Attempts,
  },
}

impl<C> State<C> where C: Clock
{
  fn timer(&mut self) -> &mut RetryTimer<C> {
    match self {
      | Self::Just(t) => t,
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
}

/// Step that manages retrying outbound messages.
///
/// See the [module documentation](crate::step::retry) for more.
#[derive(Debug)]
pub struct Retry<Inner, Buffer> {
  inner: Inner,
  buf: Buffer,
}

impl<Inner, Buffer> Default for Retry<Inner, Buffer>
  where Inner: Default,
        Buffer: Default
{
  fn default() -> Self {
    Self { inner: Inner::default(),
           buf: Buffer::default() }
  }
}

/// TODO
#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
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
  /// Error serializing a message we needed to retry
  RetrySerializingFailed(MessageToBytesError),
}

impl<E> super::Error for Error<E> where E: super::Error {}

impl<E> From<E> for Error<E> {
  fn from(e: E) -> Self {
    Error::Inner(e)
  }
}

impl<P, E, Inner, Buffer> Step<P> for Retry<Inner, Buffer>
  where Buffer: Buf<P>,
        P: Platform,
        E: super::Error,
        Inner: Step<P, PollReq = Addrd<Req<P>>, PollResp = Addrd<Resp<P>>, Error = E>
{
  type PollReq = Addrd<Req<P>>;
  type PollResp = Addrd<Resp<P>>;
  type Error = Error<E>;
  type Inner = Inner;

  fn inner(&mut self) -> &mut Self::Inner {
    &mut self.inner
  }

  fn poll_req(&mut self,
              snap: &Snapshot<P>,
              effects: &mut <P as Platform>::Effects)
              -> StepOutput<Self::PollReq, Self::Error> {
    // SERVER FLOW:
    //  * CON responses WILL     be retried
    //  * NON responses WILL NOT be retried
    //  * ACKs          WILL NOT be retried
    _try!(Result; self.buf.attempt_all::<Inner::Error>(snap.time, effects));

    let req = self.inner
                  .poll_req(snap, effects)
                  .map(|r| r.map_err(|nb| nb.map(Error::Inner)));
    let req = _try!(Option<nb::Result>; req);
    Some(Ok(req))
  }

  fn poll_resp(&mut self,
               snap: &Snapshot<P>,
               effects: &mut <P as Platform>::Effects,
               token: toad_msg::Token,
               addr: no_std_net::SocketAddr)
               -> StepOutput<Self::PollResp, Self::Error> {
    // CLIENT FLOW:
    //  * CON requests WILL be retried
    //  * NON requests WILL be retried
    _try!(Result; self.buf.attempt_all::<Inner::Error>(snap.time, effects));

    let resp =
      self.inner
          .poll_resp(snap, effects, token, addr)
          .map(|r| r.map_err(|nb| nb.map(Error::Inner)));
    let resp = _try!(Option<nb::Result>; resp);
    _try!(Result; self.buf.seen_response::<Inner::Error>(snap.time, &resp));
    Some(Ok(resp))
  }

  fn on_message_sent(&mut self,
                     snap: &platform::Snapshot<P>,
                     msg: &Addrd<platform::Message<P>>)
                     -> Result<(), Self::Error> {
    self.inner.on_message_sent(snap, msg)?;
    self.buf.store_retryables(msg, snap.time, snap.config)
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
  use crate::test::{self, ClockMock};

  fn snap_time(config: Config, time: u64) -> test::Snapshot {
    test::Snapshot { config,
                     recvd_dgram: Addrd(vec![], test::dummy_addr()),
                     time: ClockMock::instant(time * 1000) }
  }

  fn config(con_delay: u64, sec_delay: u64) -> Config {
    let con_delay = Milliseconds(con_delay);
    let sec_delay = Milliseconds(sec_delay);
    let strategy_acked_con_or_non = Strategy::Delay { min: sec_delay,
                                                      max: sec_delay };
    Config { msg: config::Msg { con_requests:
                                  config::ConRequests { unacked_retry_strategy:
                                                          Strategy::Delay { min: con_delay,
                                                                            max: con_delay },
                                                        acked_retry_strategy:
                                                          strategy_acked_con_or_non,
                                                        ..Default::default() },
                                non_requests: config::NonRequests { retry_strategy:
                                                                      strategy_acked_con_or_non,
                                                                    ..Default::default() },
                                ..Default::default() },
             ..Default::default() }
  }

  type InnerPollReq = Addrd<test::Req>;
  type InnerPollResp = Addrd<test::Resp>;

  test_step!(
    GIVEN alloc::Retry::<crate::test::Platform, Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
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
    GIVEN alloc::Retry::<crate::test::Platform, Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
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
    GIVEN alloc::Retry::<crate::test::Platform, Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
    WHEN con_sent [
      (inner.poll_resp = {
        |Snapshot {time, ..}, _, _, _| {
          let time: u64 = Milliseconds::try_from(time.duration_since_epoch()).unwrap().0;

          if time == 350 {
            // got it!
            Some(Ok(test::msg!(ACK EMPTY x.x.x.x:0000).map(Resp::from)))
          } else if time == 850 {
            // here it is!
            Some(Ok(test::msg!(NON {2 . 04} x.x.x.x:0000).map(Resp::from)))
          } else {
            None
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
          let dgram = test::msg!(CON GET x.x.x.x:1111)
                           .map(toad_msg::TryIntoBytes::try_into_bytes)
                           .map(Result::unwrap);

          assert_eq!(e, &vec![Effect::SendDgram(dgram)])
        }
      }),
      (
        poll_resp(
          snap_time(config(200, 400), 350),
          _,
          _,
          _
        ) should satisfy {
          |out| assert!(matches!(out, Some(Ok(r)) if r.data().msg.ty == Type::Ack))
        }
      ),
      (poll_resp(snap_time(config(200, 400), 550), _, _, _) should satisfy { |out| assert_eq!(out, None) }),
      (effects should satisfy {
        |e| {
          let dgram = test::msg!(CON GET x.x.x.x:1111)
                           .map(toad_msg::TryIntoBytes::try_into_bytes)
                           .map(Result::unwrap);

          assert_eq!(e, &vec![Effect::SendDgram(dgram)])
        }
      }),
      (poll_resp(snap_time(config(200, 400), 750), _, _, _) should satisfy { |out| assert_eq!(out, None) }),
      (effects should satisfy {
        |e| {
          let dgram = test::msg!(CON GET x.x.x.x:1111)
                           .map(toad_msg::TryIntoBytes::try_into_bytes)
                           .map(Result::unwrap);

          assert_eq!(e, &vec![Effect::SendDgram(dgram.clone()), Effect::SendDgram(dgram)])
        }
      }),
      (
        poll_resp(
          snap_time(config(200, 400), 850),
          _,
          _,
          _
        ) should satisfy {
          |out| assert!(matches!(out, Some(Ok(rep)) if rep.data().msg.ty == Type::Non))
        }
      ),
      (poll_resp(snap_time(config(200, 400), 10_000), _, _, _) should satisfy { |out| assert_eq!(out, None) }),
      (effects should satisfy {
        |e| {
          let dgram = test::msg!(CON GET x.x.x.x:1111)
                           .map(toad_msg::TryIntoBytes::try_into_bytes)
                           .map(Result::unwrap);

          assert_eq!(e, &vec![Effect::SendDgram(dgram.clone()), Effect::SendDgram(dgram)])
        }
      })
    ]
  );

  // TODO:
  //  * ACKs are not retried
  //  * NON responses are not retried
  //  * NON requests are retried
  //  * CON responses are retried
}
