use embedded_time::clock::Error;
use embedded_time::{Clock, Instant};

/// Data associated with a timestamp
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Stamped<C: Clock, T>(pub T, pub Instant<C>);

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
  pub fn map<R>(self, f: impl FnOnce(T) -> R) -> Stamped<C, R> {
    Stamped(f(self.0), self.1)
  }

  /// TODO
  pub fn find_latest(winner: Option<Stamped<C, T>>, cur: Stamped<C, T>) -> Option<Stamped<C, T>> {
    Some(winner.filter(|Stamped(_, winner)| winner > &cur.time()).unwrap_or(cur))
  }
}
