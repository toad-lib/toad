use core::fmt::Display;
use core::ops::{Deref, DerefMut};

use crate::Array;

/// A writeable byte buffer
///
/// (allows using `write!` and `format!` without allocations)
///
/// ```
/// use core::fmt::Write as _;
///
/// use toad_common::{Array, Writable};
///
/// let mut faux_string = Writable::from(tinyvec::ArrayVec::<[u8; 16]>::new());
/// write!(faux_string, "{}", 123).unwrap();
///
/// assert_eq!(faux_string.as_str(), "123");
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct Writable<A: Array<Item = u8>>(A);

impl<A: Array<Item = u8>> Writable<A> {
  /// Attempt to read the data in the buffer
  /// as a UTF8 string slice
  pub fn as_str(&self) -> &str {
    core::str::from_utf8(self).unwrap()
  }

  /// Get the collection wrapped by this `Writable`
  pub fn unwrap(self) -> A {
    self.0
  }
}

impl<A> Display for Writable<A> where A: Array<Item = u8>
{
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "{}", self.as_str())
  }
}

impl<A: Array<Item = u8>> From<A> for Writable<A> {
  fn from(a: A) -> Self {
    Self(a)
  }
}

impl<A: Array<Item = u8>> Deref for Writable<A> {
  type Target = A;

  fn deref(&self) -> &A {
    &self.0
  }
}

impl<A: Array<Item = u8>> DerefMut for Writable<A> {
  fn deref_mut(&mut self) -> &mut A {
    &mut self.0
  }
}

impl<A: Array<Item = u8>> AsRef<str> for Writable<A> {
  fn as_ref(&self) -> &str {
    self.as_str()
  }
}

impl<A: Array<Item = u8>> core::fmt::Write for Writable<A> {
  fn write_str(&mut self, s: &str) -> core::fmt::Result {
    match A::CAPACITY {
      | Some(max) if max < self.len() + s.len() => Err(core::fmt::Error),
      | _ => {
        self.extend(s.bytes());
        Ok(())
      },
    }
  }
}
