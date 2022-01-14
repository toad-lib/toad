use embedded_time::{duration::{Milliseconds, Duration}, Clock, Instant, TimeInt};

use crate::result_ext::ResultExt;

type ClockResult<T> = Result<T, embedded_time::clock::Error>;

/// TODO
#[derive(Debug, Clone, Copy)]
pub struct Retry<C: Clock<T = u64>> {
  start: Instant<C>,
  clock: C,
  strategy: Strategy,
  attempts: u16,
  max_attempts: u16,
}

/// TODO
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum Attempt {
  /// Attempts have been exhausted and the work that is
  /// being retried should be considered poisoned.
  NoMoreAttempts,
  /// A retry should be performed
  Retry,
  /// Not ready to retry
  WouldBlock,
}

impl<C: Clock<T = u64>> Retry<C> {
  /// Create a new retrier
  pub fn try_new(clock: C, strategy: Strategy, max_attempts: u16) -> ClockResult<Self> {
    clock.try_now().map(|start| Self {start, clock, strategy, max_attempts, attempts: 0})
  }

  /// Ask the retrier if we should retry
  pub fn attempt(&mut self) -> ClockResult<Attempt> {
    if self.attempts >= self.max_attempts {
      Ok(Attempt::NoMoreAttempts)
    } else {
      self.clock
          .try_now()
          .map(|now| now - self.start)
          .map(|elapsed| self.strategy.is_ready(elapsed.try_into().unwrap(), self.attempts))
          .map(|ready| if ready {self.attempts += 1; Attempt::Retry} else {Attempt::WouldBlock})
    }
  }
}

/// Strategy to employ when retrying
#[derive(Debug, Clone, Copy)]
pub enum Strategy {
  /// After each failed attempt, double the delay before retrying again.
  Exponential(Milliseconds<u64>),
  /// Wait a fixed delay between attempts.
  ///
  /// Field 1 is the maximum number of attempts
  Delay(Milliseconds<u64>),
}

impl Strategy {
  /// Check if the strategy says an appropriate time has passed
  pub fn is_ready(&self, time_passed: Milliseconds<u64>, attempts: u16) -> bool {
    if attempts == 0 {
      return true;
    }

    match self {
      Self::Delay(dur) => time_passed.0 >= (dur.0 * attempts as u64),
      Self::Exponential(dur) => time_passed.0 >= Self::total_delay_exp(*dur, attempts),
    }
  }

  fn total_delay_exp(init: Milliseconds<u64>, attempts: u16) -> u64 {
    // att: 0 = 0
    // att: 1 = init
    // att: 2 = init + init * 2
    // att: 3 = init + init * 2 + init * 4
    match attempts {
      0 => 0,
      1 => init.0,
      n => init.0 + (1..n).map(|n| init.0 * 2u64.pow(n as u32)).sum::<u64>()
    }
  }
}

#[cfg(test)]
mod test {
  use embedded_time::rate::Fraction;

use super::*;

  pub struct FakeClock(pub *const u64);
  impl FakeClock {
    pub fn new(time_ptr: *const u64) -> Self {Self(time_ptr)}
  }

  impl Clock for FakeClock {
    type T = u64;

    const SCALING_FACTOR: Fraction = Fraction::new(1, 1000);

    fn try_now(&self) -> Result<Instant<Self>, embedded_time::clock::Error> {
      unsafe {
        Ok(Instant::new(*self.0))
      }
    }
}

  #[test]
  fn retrier() {
    #![allow(unused_assignments)]

    let mut time_millis = 0u64;
    let clock = FakeClock::new(&time_millis as *const _);
    let mut retry = Retry::try_new(clock, Strategy::Delay(Milliseconds(1000)), 5).unwrap();

    assert_eq!(retry.attempt().unwrap(), Attempt::Retry); // Attempt 1

    time_millis = 999;
    assert_eq!(retry.attempt().unwrap(), Attempt::WouldBlock);

    time_millis = 1000;
    assert_eq!(retry.attempt().unwrap(), Attempt::Retry); // Attempt 2

    time_millis = 1999;
    assert_eq!(retry.attempt().unwrap(), Attempt::WouldBlock);

    time_millis = 2000;
    assert_eq!(retry.attempt().unwrap(), Attempt::Retry); // Attempt 3

    time_millis = 10_000;
    assert_eq!(retry.attempt().unwrap(), Attempt::Retry); // Attempt 4
    assert_eq!(retry.attempt().unwrap(), Attempt::Retry); // Attempt 5
    assert_eq!(retry.attempt().unwrap(), Attempt::NoMoreAttempts); // Attempt 6
  }

  #[test]
  fn delay_waits() {
    let strat = Strategy::Delay(Milliseconds(100));

    assert!(strat.is_ready(Milliseconds(0), 0));

    assert!(!strat.is_ready(Milliseconds(99), 1));
    assert!(strat.is_ready(Milliseconds(100), 1));

    assert!(!strat.is_ready(Milliseconds(199), 2));
    assert!(strat.is_ready(Milliseconds(200), 2));

    assert!(!strat.is_ready(Milliseconds(299), 3));
    assert!(strat.is_ready(Milliseconds(300), 3));
  }

  #[test]
  fn exp_calculation() {
    let init = Milliseconds(100);
    assert_eq!(Strategy::total_delay_exp(init, 0), 0);
    assert_eq!(Strategy::total_delay_exp(init, 1), 100);
    assert_eq!(Strategy::total_delay_exp(init, 2), 300);
    assert_eq!(Strategy::total_delay_exp(init, 3), 700);
  }

  #[test]
  fn exp_waits() {
    let strat = Strategy::Exponential(Milliseconds(100));

    assert!(strat.is_ready(Milliseconds(0), 0));

    assert!(!strat.is_ready(Milliseconds(99), 1));
    assert!( strat.is_ready(Milliseconds(100), 1));

    assert!(!strat.is_ready(Milliseconds(299), 2));
    assert!( strat.is_ready(Milliseconds(300), 2));

    assert!(!strat.is_ready(Milliseconds(699), 3));
    assert!( strat.is_ready(Milliseconds(700), 3));
  }
}
