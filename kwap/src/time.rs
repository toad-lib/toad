use embedded_time::clock::Error;
use embedded_time::{Clock, Instant};

/// Data associated with a timestamp
pub struct Stamped<C: Clock, T>(pub T, pub Instant<C>);

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
}

/// TODO
pub trait StampedIterator<C: Clock, T> {
  fn latest(self) -> Option<Stamped<C, T>>;
}

impl<I: Iterator<Item = Stamped<C, T>>, C: Clock, T> StampedIterator<C, T> for I {
  fn latest(self) -> Option<Stamped<C, T>> {
    self.fold(None, |winner, new| {
          Some(winner.filter(|Stamped(_, winner)| winner > &new.time()).unwrap_or(new))
        })
  }
}
