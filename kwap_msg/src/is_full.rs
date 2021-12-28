#[cfg(feature = "alloc")]
use std_alloc::vec::Vec;
use tinyvec::{ArrayVec, Array};

/// Data structures that can be initialized with some capacity and may have a hard capacity limit.
///
/// The default implementation represents a collection of infinite capacity that does not reserve
/// any space when `with_capacity` is invoked.
///
/// # Examples
/// ```
/// use kwap_msg::Capacity;
///
/// // Newtype because there's a provided implementation of `Capacity` for `Vec`.
/// struct HeapArray<T>(Vec<T>);
///
/// // - default `max_capacity` because Vec will grow to fit new elements
/// // - implement `with_capacity` however because Vec can be created with space reserved
/// impl<T> Capacity for HeapArray<T> {
///   fn with_capacity(n: usize) -> Self { Self(Vec::with_capacity(n)) }
/// }
///
/// // - implement `max_capacity` because arrays have hard element limits
/// // - default `with_capacity` because the space will be reserved when an array is created
/// impl<const N: usize> Capacity for [u8; N] {
///   fn max_capacity() -> usize {N}
/// }
///
/// struct LinkedList<T> {
///   // listy things
/// # items: Vec<T>,
/// }
///
/// // - default `max_capacity` because linked lists grow to fit new elements
/// // - default `with_capacity` because linked lists do not "reserve" capacity;
/// //   their items aren't stored on contiguous memory layouts
/// impl<T> Capacity for LinkedList<T> {}
/// ```
pub trait Capacity: Default {
  /// Get the max size that this data structure can acommodate.
  ///
  /// By default, this returns `usize::MAX` and can be left unimplemented for dynamic collections.
  ///
  /// However, for fixed-size collections this method must be implemented.
  fn max_capacity(&self) -> usize {
    usize::MAX
  }

  /// Create an instance of the collection with a given capacity.
  ///
  /// Used to reserve some contiguous space, e.g. [`Vec::with_capacity`]
  ///
  /// By default, invokes `Default::default`
  fn with_capacity(_: usize) -> Self {
    Default::default()
  }
}

/// Given a structure that has a [`Capacity`] and a [`GetSize`], check whether it has capacity remaining
pub trait IsFull {
  /// Is there no room left in this collection?
  fn is_full(&self) -> bool;
}

impl<T: Capacity + crate::GetSize> IsFull for T {
  fn is_full(&self) -> bool {
    self.get_size() >= self.max_capacity()
  }
}

#[cfg(feature = "alloc")]
impl<T> Capacity for Vec<T> {
  fn with_capacity(n: usize) -> Self {Self::with_capacity(n)}
}

impl<A: Array> Capacity for ArrayVec<A> {
  fn max_capacity(&self) -> usize {
    self.capacity()
  }
}
