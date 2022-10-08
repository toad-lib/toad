use core::ops::RangeInclusive;

use embedded_time::duration::Milliseconds;
use embedded_time::{Clock, Instant};
use rand::{Rng, SeedableRng};
use toad_common::prelude::*;

/// A non-blocking timer that allows a fixed-delay or exponential-backoff retry,
/// that lives alongside some operation to retry.
///
/// It does not _contain_ the work to be done (e.g. `Box<fn()>`) because
/// we don't have the luxury of a memory allocator :)
///
/// ```
/// use embedded_time::clock::Clock;
/// use embedded_time::duration::Milliseconds;
/// use toad::retry;
///
/// # main();
/// fn main() {
///   let mut called = false;
///   let mut fails_once = || -> Result<(), ()> {
///     // ...
///     # if !called {
///     #   called = true;
///     #   Err(())
///     # } else {
///     #   Ok(())
///     # }
///   };
///
///   let clock = toad::std::Clock::new();
///   let now = || clock.try_now().unwrap();
///   let strategy = retry::Strategy::Delay { min: Milliseconds(1),
///                                           max: Milliseconds(2) };
///   let mut retry = retry::RetryTimer::new(now(), strategy, retry::Attempts(2));
///
///   while let Err(_) = fails_once() {
///     match nb::block!(retry.what_should_i_do(now())) {
///       | Ok(retry::YouShould::Retry) => continue,
///       | Ok(retry::YouShould::Cry) => panic!("no more attempts! it failed more than once!!"),
///       | Err(clock_err) => unreachable!(),
///     }
///   }
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct RetryTimer<C: Clock<T = u64>> {
  start: Instant<C>,
  init: Milliseconds<u64>,
  strategy: Strategy,
  attempts: Attempts,
  max_attempts: Attempts,
}

/// A number of attempts
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Attempts(pub u16);

/// Result of [`RetryTimer.what_should_i_do`].
///
/// This tells you if a retry should be attempted or not.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum YouShould {
  /// Attempts have been exhausted and the work that is
  /// being retried should be considered poisoned.
  Cry,
  /// A retry should be performed
  Retry,
}

impl<C: Clock<T = u64>> RetryTimer<C> {
  /// Create a new retrier
  pub fn new(start: Instant<C>, strategy: Strategy, max_attempts: Attempts) -> Self {
    Self { start,
           strategy,
           init: if strategy.has_jitter() {
             let mut rand =
               Ok(start.duration_since_epoch()).bind(Milliseconds::try_from)
                                               .map(|Milliseconds(ms)| {
                                                 rand_chacha::ChaCha8Rng::seed_from_u64(ms)
                                               })
                                               .unwrap();

             Milliseconds(rand.gen_range(strategy.range()))
           } else {
             Milliseconds(*strategy.range().start())
           },
           max_attempts,
           attempts: Attempts(1) }
  }

  /// When the thing we keep trying fails, invoke this to
  /// tell the retrytimer "it failed again! what do I do??"
  ///
  /// Returns `nb::Error::WouldBlock` when we have not yet
  /// waited the appropriate amount of time to retry.
  pub fn what_should_i_do(&mut self,
                          now: Instant<C>)
                          -> nb::Result<YouShould, core::convert::Infallible> {
    if self.attempts >= self.max_attempts {
      Ok(YouShould::Cry)
    } else {
      let ready = self.is_ready((now - self.start).try_into().unwrap(), self.attempts.0);
      if ready {
        self.attempts.0 += 1;
        Ok(YouShould::Retry)
      } else {
        Err(nb::Error::WouldBlock)
      }
    }
  }

  /// Check if the strategy says an appropriate time has passed
  pub fn is_ready(&self, Milliseconds(time_passed): Milliseconds<u64>, attempts: u16) -> bool {
    if attempts == 0 {
      return true;
    }

    match self.strategy {
      | Strategy::Delay { .. } => time_passed >= (self.init.0 * attempts as u64),
      | Strategy::Exponential { .. } => {
        time_passed >= Strategy::total_delay_exp(self.init, attempts)
      },
    }
  }
}

/// Strategy to employ when retrying
#[derive(Debug, Clone, Copy)]
pub enum Strategy {
  /// Generate a random delay between `min` and `max`,
  /// and wait until this delay has passed between attempts.
  ///
  /// After each failed attempt, double the delay before retrying again.
  Exponential {
    /// Minimum (inclusive) delay for second attempt
    init_min: Milliseconds<u64>,
    /// Maximum (inclusive) delay for second attempt
    init_max: Milliseconds<u64>,
  },
  /// Generate a random delay between `min` and `max`,
  /// and wait until this delay has passed between attempts.
  Delay {
    /// Minimum (inclusive) delay for attempts
    min: Milliseconds<u64>,
    /// Maximum (inclusive) delay for attempts
    max: Milliseconds<u64>,
  },
}

impl Strategy {
  /// Are min & max delays the same? if so, we should probably skip the random number generation.
  pub fn has_jitter(&self) -> bool {
    let rng = self.range();
    rng.start() != rng.end()
  }

  /// Get the min & max durations as an inclusive range
  pub fn range(&self) -> RangeInclusive<u64> {
    match self {
      | &Self::Delay { min: Milliseconds(min),
                       max: Milliseconds(max), } => (min..=max),

      | &Self::Exponential { init_min: Milliseconds(min),
                             init_max: Milliseconds(max), } => (min..=max),
    }
  }

  /// Get the amount of time this strategy will take if all attempts fail
  pub fn max_time(&self, max_attempts: Attempts) -> Milliseconds<u64> {
    Milliseconds(match self {
                   | Self::Exponential { init_max, .. } => {
                     Self::total_delay_exp(*init_max, max_attempts.0)
                   },
                   | Self::Delay { max: Milliseconds(max),
                                   .. } => max * max_attempts.0 as u64,
                 })
  }

  /// Given the initial delay and number of attempts that have been performed,
  /// yields the delay until the next retry should be attempted.
  const fn total_delay_exp(Milliseconds(init): Milliseconds<u64>, attempt: u16) -> u64 {
    // | attempt | total delay      |
    // | 1       | init             |
    // | 2       | init * 2         |
    // | 3       | init * 4         |
    // | ...     | ...              |
    // | n       | init * 2^n       |
    init * 2u64.pow((attempt - 1) as u32)
  }
}

#[cfg(test)]
mod test {
  use embedded_time::rate::Fraction;

  use super::*;

  pub struct FakeClock(pub *const u64);
  impl FakeClock {
    pub fn new(time_ptr: *const u64) -> Self {
      Self(time_ptr)
    }
  }

  impl Clock for FakeClock {
    type T = u64;

    const SCALING_FACTOR: Fraction = Fraction::new(1, 1000);

    fn try_now(&self) -> Result<Instant<Self>, embedded_time::clock::Error> {
      unsafe { Ok(Instant::new(*self.0)) }
    }
  }

  #[test]
  fn delay_retrier() {
    #![allow(unused_assignments)]

    let mut time_millis = 0u64;
    let clock = FakeClock::new(&time_millis as *const _);
    let now = || clock.try_now().unwrap();
    let mut retry = RetryTimer::new(now(),
                                    Strategy::Delay { min: Milliseconds(1000),
                                                      max: Milliseconds(1000) },
                                    Attempts(5));

    // attempt 1 happens before asking what_should_i_do

    time_millis = 999;
    assert_eq!(retry.what_should_i_do(now()).unwrap_err(),
               nb::Error::WouldBlock);

    time_millis = 1000;
    assert_eq!(retry.what_should_i_do(now()).unwrap(), YouShould::Retry);
    // Fails again (attempt 2)

    time_millis = 1999;
    assert_eq!(retry.what_should_i_do(now()).unwrap_err(),
               nb::Error::WouldBlock);

    time_millis = 2000;

    assert_eq!(retry.what_should_i_do(now()).unwrap(), YouShould::Retry);
    // Fails again (attempt 3)

    time_millis = 10_000;
    assert_eq!(retry.what_should_i_do(now()).unwrap(), YouShould::Retry);
    // Fails again (attempt 4)

    // TODO: this is a logic error but not totally worth a ton of attention
    // hypothetically we could fail, wait 10 seconds, then fail 5 times super fast
    // because the retrier only tracks now vs total time waited, not time since last
    // attempt.
    assert_eq!(retry.what_should_i_do(now()).unwrap(), YouShould::Retry);
    // Fails again (attempt 5)

    assert_eq!(retry.what_should_i_do(now()).unwrap(), YouShould::Cry);
  }

  #[test]
  fn exponential_retrier() {
    #![allow(unused_assignments)]

    let mut time_millis = 0u64;
    let clock = FakeClock::new(&time_millis as *const _);
    let now = || clock.try_now().unwrap();
    let mut retry = RetryTimer::new(now(),
                                    Strategy::Exponential { init_min: Milliseconds(1000),
                                                            init_max: Milliseconds(1000) },
                                    Attempts(6));

    // attempt 1 happens before asking what_should_i_do

    time_millis = 999;
    println!("{}", time_millis);
    assert_eq!(retry.what_should_i_do(now()).unwrap_err(),
               nb::Error::WouldBlock);

    time_millis = 1000;
    println!("{}", time_millis);
    assert_eq!(retry.what_should_i_do(now()).unwrap(), YouShould::Retry);

    time_millis = 1999;
    println!("{}", time_millis);
    assert_eq!(retry.what_should_i_do(now()).unwrap_err(),
               nb::Error::WouldBlock);

    time_millis = 2000;
    println!("{}", time_millis);
    assert_eq!(retry.what_should_i_do(now()).unwrap(), YouShould::Retry);

    time_millis = 3999;
    println!("{}", time_millis);
    assert_eq!(retry.what_should_i_do(now()).unwrap_err(),
               nb::Error::WouldBlock);

    time_millis = 4000;
    println!("{}", time_millis);
    assert_eq!(retry.what_should_i_do(now()).unwrap(), YouShould::Retry);

    time_millis = 8_000;
    println!("{}", time_millis);
    assert_eq!(retry.what_should_i_do(now()).unwrap(), YouShould::Retry);

    time_millis = 16_000;
    println!("{}", time_millis);
    assert_eq!(retry.what_should_i_do(now()).unwrap(), YouShould::Retry);

    assert_eq!(retry.what_should_i_do(now()).unwrap(), YouShould::Cry);
  }

  #[test]
  fn exp_calculation() {
    let init = Milliseconds(100);
    assert_eq!(Strategy::total_delay_exp(init, 1), 100);
    assert_eq!(Strategy::total_delay_exp(init, 2), 200);
    assert_eq!(Strategy::total_delay_exp(init, 3), 400);
  }
}
