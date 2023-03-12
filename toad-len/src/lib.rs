//! This microcrate contains a `Len` trait that provides capacity and runtime length
//! for collections.

// docs
#![doc(html_root_url = "https://docs.rs/toad-len/0.1.0")]
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

use core::hash::Hash;

#[cfg(feature = "std")]
use std::collections::HashMap;

#[cfg(feature = "alloc")]
use std_alloc::collections::BTreeMap;

/// Get the runtime size of some data structure
pub trait Len {
  /// The maximum number of elements that this data structure can acommodate.
  const CAPACITY: Option<usize>;

  /// Get the runtime size (in bytes) of a struct
  ///
  /// For collections this is always equivalent to calling an inherent `len` method.
  ///
  /// ```
  /// use toad_len::Len;
  ///
  /// assert_eq!(Len::len(&vec![1u8, 2]), 2)
  /// ```
  fn len(&self) -> usize;

  /// Check if the runtime size is zero
  ///
  /// ```
  /// use toad_len::Len;
  ///
  /// assert!(Len::is_empty(&Vec::<u8>::new()))
  /// ```
  fn is_empty(&self) -> bool {
    self.len() == 0
  }

  /// Is there no room left in this collection?
  ///
  /// ```
  /// use toad_len::Len;
  ///
  /// let array = tinyvec::ArrayVec::<[u8; 2]>::from([1, 2]);
  ///
  /// assert!(Len::is_full(&array))
  /// ```
  fn is_full(&self) -> bool;
}

#[cfg(feature = "alloc")]
impl<T> Len for std_alloc::vec::Vec<T> {
  const CAPACITY: Option<usize> = None;

  fn len(&self) -> usize {
    self.len()
  }

  fn is_full(&self) -> bool {
    false
  }
}

impl<A: tinyvec::Array> Len for tinyvec::ArrayVec<A> {
  const CAPACITY: Option<usize> = Some(A::CAPACITY);

  fn len(&self) -> usize {
    self.len()
  }

  fn is_full(&self) -> bool {
    self.len() >= self.capacity()
  }
}

#[cfg(feature = "std")]
impl<K: Eq + Hash, V> Len for HashMap<K, V> {
  const CAPACITY: Option<usize> = None;

  fn len(&self) -> usize {
    self.len()
  }

  fn is_full(&self) -> bool {
    false
  }
}

#[cfg(feature = "alloc")]
impl<K, V> Len for BTreeMap<K, V> {
  const CAPACITY: Option<usize> = None;

  fn len(&self) -> usize {
    self.len()
  }

  fn is_full(&self) -> bool {
    false
  }
}
