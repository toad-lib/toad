use tinyvec::{ArrayVec, Array};

/// Encapsulates the capacity of some data structure with a variable internal size, and a max size.
///
/// The default implementation allows for a collection of infinite capacity and does not pre-allocate
/// any space when `with_capacity` is invoked.
pub trait Capacity: Default {
  /// Get the max size of this data structure.
  ///
  /// By default, this returns `usize::MAX` and can be left unimplemented for dynamic collections.
  ///
  /// However, for fixed-size applications this method must be implemented.
  fn capacity(&self) -> usize {
    usize::MAX
  }

  /// Create an instance of the collection with a given capacity.
  ///
  /// Used to create dynamic allocating collections with a known initial size
  ///
  /// By default, invokes `Default::default`
  fn with_capacity(_: usize) -> Self {
    Default::default()
  }
}

/// Given a structure that has a [`Capacity`] and a [`GetSize`], check whether it has capacity remaining
pub trait IsFull {
  fn is_full(&self) -> bool;
}

impl<T: Capacity + crate::GetSize> IsFull for T {
  fn is_full(&self) -> bool {
    self.get_size() >= self.capacity()
  }
}

#[cfg(feature = "alloc")]
impl<T> Capacity for std_alloc::vec::Vec<T> {
  fn with_capacity(n: usize) -> Self {Self::with_capacity(n)}
}

impl<A: Array> Capacity for ArrayVec<A> {
  fn capacity(&self) -> usize {
    self.capacity()
  }
}
