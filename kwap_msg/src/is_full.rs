#[allow(unused_imports)]
use crate::GetSize;
#[cfg(feature = "alloc")]
use std_alloc::vec::Vec;
use tinyvec::{ArrayVec, Array};

/// Data structures that may be created by reserving some amount of space
pub trait Reserve: Default {
  /// Create an instance of the collection with a given capacity.
  ///
  /// Used to reserve some contiguous space, e.g. [`Vec::with_capacity`]
  ///
  /// By default, invokes `Default::default`
  fn reserve(_: usize) -> Self {
    Default::default()
  }
}

#[cfg(feature = "alloc")]
impl<T> Reserve for Vec<T> {
  fn reserve(n: usize) -> Self {Self::with_capacity(n)}
}

impl<A: Array> Reserve for ArrayVec<A> {}
