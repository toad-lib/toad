#[cfg(feature = "alloc")]
use std_alloc::vec::Vec;

use tinyvec::ArrayVec;

/// Insert items into collections
pub trait Insert<T>: crate::GetSize {
  /// Insert a value at a particular index of a collection.
  fn insert_at(&mut self, index: usize, value: T);

  /// Insert a value to the end of a collection.
  fn push(&mut self, value: T) {
    self.insert_at(self.get_size(), value)
  }
}

#[cfg(feature = "alloc")]
impl<T> Insert<T> for Vec<T> {
  fn insert_at(&mut self, index: usize, value: T) {
    self.insert(index, value);
  }

  fn push(&mut self, value: T) {
    self.push(value)
  }
}

impl<A: tinyvec::Array> Insert<A::Item> for ArrayVec<A> {
  fn insert_at(&mut self, index: usize, value: A::Item) {
    self.insert(index, value);
  }

  fn push(&mut self, value: A::Item) {
    self.push(value)
  }
}
