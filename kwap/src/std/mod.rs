#![allow(clippy::many_single_char_names)]

use embedded_time::rate::Fraction;

/// TODO
pub mod net;
pub use net::*;

/// Implement [`embedded_time::Clock`] using [`std::time`] primitives
#[derive(Debug, Clone, Copy)]
pub struct Clock(std::time::Instant);

impl Default for Clock {
  fn default() -> Self {
    Self::new()
  }
}

impl Clock {
  /// Create a new clock
  pub fn new() -> Self {
    Self(std::time::Instant::now())
  }
}

impl embedded_time::Clock for Clock {
  type T = u64;

  // nanoseconds
  const SCALING_FACTOR: Fraction = Fraction::new(1, 1_000_000_000);

  fn try_now(&self) -> Result<embedded_time::Instant<Self>, embedded_time::clock::Error> {
    let now = std::time::Instant::now();
    let elapsed = now.duration_since(self.0);
    Ok(embedded_time::Instant::new(elapsed.as_nanos() as u64))
  }
}
