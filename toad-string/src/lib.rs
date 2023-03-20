//! This microcrate contains the stack-allocated `String` struct used by the
//! [`toad`](https://github.com/toad-lib/toad) CoAP runtime / ecosystem.

// docs
#![doc(html_root_url = "https://docs.rs/toad-string/0.0.0")]
#![cfg_attr(any(docsrs, feature = "docs"), feature(doc_cfg))]
// -
// style
#![allow(clippy::unused_unit)]
// -
// deny
#![deny(missing_docs)]
#![deny(missing_debug_implementations)]
#![deny(missing_copy_implementations)]
#![cfg_attr(not(test), deny(unsafe_code))]
// -
// warnings
#![cfg_attr(not(test), warn(unreachable_pub))]
// -
// features
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc as std_alloc;

use core::fmt::Display;

use tinyvec::ArrayVec;
use toad_writable::Writable;

/// Stack-allocated mutable string with capacity of 1KB
///
/// ```
/// use toad_string::String;
///
/// assert_eq!(String::<16>::from("ron stampler").as_str(), "ron stampler")
/// ```
#[derive(Debug, Copy, Clone, Default)]
pub struct String<const N: usize>(Writable<ArrayVec<[u8; N]>>);

impl<const N: usize> String<N> {
  /// Alias for [`AsRef`]
  pub fn as_str(&self) -> &str {
    self.as_ref()
  }

  /// Resize the String to a new length
  ///
  /// If `M` is less than `N`, the extra bytes are
  /// discarded.
  pub fn resize<const M: usize>(&mut self) -> String<M> {
    let mut bytes = self.0.unwrap();
    bytes.truncate(M);
    String(Writable::from(self.as_writable().drain(..).collect::<ArrayVec<[u8; M]>>()))
  }

  /// Alias for [`AsRef`]
  pub fn as_bytes(&self) -> &[u8] {
    self.as_ref()
  }

  /// Get a mutable reference to the inner writable buffer
  pub fn as_writable(&mut self) -> &mut Writable<ArrayVec<[u8; N]>> {
    &mut self.0
  }
}

impl<const N: usize> Display for String<N> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "{}", self.as_str())
  }
}

impl<const N: usize> PartialEq for String<N> {
  fn eq(&self, other: &Self) -> bool {
    self.0.as_str() == other.0.as_str()
  }
}

impl<const N: usize> Eq for String<N> {}

impl<const N: usize> core::fmt::Write for String<N> {
  fn write_str(&mut self, s: &str) -> core::fmt::Result {
    self.0.write_str(s)
  }
}

impl<'a, const N: usize> From<&'a str> for String<N> {
  fn from(s: &'a str) -> Self {
    let mut arr = Writable::default();
    ArrayVec::extend_from_slice(&mut arr, s.as_bytes());

    Self(arr)
  }
}

impl<const N: usize> AsRef<str> for String<N> {
  fn as_ref(&self) -> &str {
    self.0.as_str()
  }
}

impl<const N: usize> AsRef<[u8]> for String<N> {
  fn as_ref(&self) -> &[u8] {
    self.0.as_str().as_bytes()
  }
}
