use embedded_time::clock::Error;
use embedded_time::Instant;

use crate::todo::String;

/// A duration, in milliseconds
pub type Millis = embedded_time::duration::Milliseconds<u64>;

/// Supertrait of [`embedded_time::Clock`] pinning the
/// type of "ticks" to u64
pub trait Clock: embedded_time::Clock<T = u64> {}
impl<C: embedded_time::Clock<T = u64>> Clock for C {}

/// Timeout configuration allowing for "never time out" as an option
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
pub enum Timeout {
  /// Timeout after some number of milliseconds has elapsed
  Millis(u64),
  /// Never time out
  Never,
}

/// Data associated with a timestamp
pub struct Stamped<C: Clock, T>(pub T, pub Instant<C>);

impl<C: Clock, T: core::fmt::Debug> core::fmt::Debug for Stamped<C, T> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    use core::fmt::Write;

    let mut instant = String::<100>::default();
    write!(instant,
           "<{}ms since epoch>",
           Millis::try_from(self.1.duration_since_epoch()).unwrap())?;

    f.debug_tuple("Stamped")
     .field(&self.0)
     .field(&instant)
     .finish()
  }
}

impl<C: Clock, T: PartialEq> PartialEq for Stamped<C, T> {
  fn eq(&self, other: &Self) -> bool {
    self.0 == other.0 && self.1 == other.1
  }
}

impl<C: Clock, T: Eq> Eq for Stamped<C, T> {}

impl<C: Clock, T: PartialOrd> PartialOrd for Stamped<C, T> {
  fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
    use core::cmp::Ordering;

    match self.0.partial_cmp(&other.0) {
      | Some(Ordering::Equal) => Some(self.1.cmp(&other.1)),
      | ne => ne,
    }
  }
}

impl<C: Clock, T: Ord> Ord for Stamped<C, T> {
  fn cmp(&self, other: &Self) -> core::cmp::Ordering {
    use core::cmp::Ordering;

    match self.0.cmp(&other.0) {
      | Ordering::Equal => self.1.cmp(&other.1),
      | ne => ne,
    }
  }
}

impl<C: Clock, T: Default> Default for Stamped<C, T> {
  fn default() -> Self {
    Self(T::default(), Instant::new(0))
  }
}

impl<C: Clock, T: Clone> Clone for Stamped<C, T> {
  fn clone(&self) -> Self {
    Self(self.0.clone(), self.1)
  }
}

impl<C: Clock, T: Copy> Copy for Stamped<C, T> {}

impl<C: Clock, T> Stamped<C, T> {
  /// TODO
  pub fn new(clock: &C, t: T) -> Result<Self, Error> {
    clock.try_now().map(|now| Self(t, now))
  }

  /// TODO
  pub fn as_ref(&self) -> Stamped<C, &T> {
    Stamped(&self.0, self.1)
  }

  /// TODO
  pub fn as_mut(&mut self) -> Stamped<C, &mut T> {
    Stamped(&mut self.0, self.1)
  }

  /// TODO
  pub fn data(&self) -> &T {
    &self.0
  }

  /// TODO
  pub fn time(&self) -> Instant<C> {
    self.1
  }

  /// TODO
  pub fn discard_timestamp(self) -> T {
    self.0
  }

  /// TODO
  pub fn map<R>(self, f: impl FnOnce(T) -> R) -> Stamped<C, R> {
    Stamped(f(self.0), self.1)
  }

  /// TODO
  pub fn find_latest(winner: Option<Stamped<C, T>>, cur: Stamped<C, T>) -> Option<Stamped<C, T>> {
    Some(winner.filter(|winner| winner.time() > cur.time())
               .unwrap_or(cur))
  }
}
