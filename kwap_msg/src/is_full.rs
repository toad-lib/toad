pub(crate) trait IsFull {
  fn is_full(&self) -> bool;
}

#[cfg(feature = "alloc")]
impl<T> IsFull for std_alloc::vec::Vec<T> {
  fn is_full(&self) -> bool {
    self.capacity() == self.len()
  }
}

impl<A: tinyvec::Array> IsFull for tinyvec::ArrayVec<A> {
  fn is_full(&self) -> bool {
    self.capacity() == self.len()
  }
}
