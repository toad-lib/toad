use crate::*;

/// Get the runtime size (in bytes) of a struct
///
/// ## Note
/// For collections this just yields the number of elements ([`Vec::len`], [`tinyvec::ArrayVec::len`]),
/// and when the collection is over [`u8`]s,
/// then `get_size` represents the number of bytes in the collection.
pub trait GetSize {
  /// Get the runtime size (in bytes) of a struct
  ///
  /// For collections this is always equivalent to calling an inherent `len` method.
  ///
  /// ```
  /// use kwap_msg::GetSize;
  ///
  /// assert_eq!(vec![1u8, 2].get_size(), 2)
  /// ```
  fn get_size(&self) -> usize;

  /// Get the max size that this data structure can acommodate.
  ///
  /// By default, this returns `None` and can be left unimplemented for dynamic collections.
  ///
  /// However, for fixed-size collections this method must be implemented.
  fn max_size(&self) -> Option<usize>;

  /// Check if the runtime size is zero
  ///
  /// ```
  /// use kwap_msg::GetSize;
  ///
  /// assert!(Vec::<u8>::new().size_is_zero())
  /// ```
  fn size_is_zero(&self) -> bool {
    self.get_size() == 0
  }

  /// Is there no room left in this collection?
  fn is_full(&self) -> bool {
    self.max_size().map(|max| self.get_size() >= max).unwrap_or(false)
  }
}

#[cfg(feature = "alloc")]
impl<T> GetSize for std_alloc::vec::Vec<T> {
  fn get_size(&self) -> usize {
    self.len()
  }

  fn max_size(&self) -> Option<usize> {
    None
  }
}

impl<A: tinyvec::Array> GetSize for tinyvec::ArrayVec<A> {
  fn get_size(&self) -> usize {
    self.len()
  }

  fn max_size(&self) -> Option<usize> {
    Some(A::CAPACITY)
  }
}

impl<P: Collection<u8>, O: Collection<u8>, Os: Collection<Opt<O>>> GetSize for Message<P, O, Os>
  where for<'b> &'b P: IntoIterator<Item = &'b u8>,
        for<'b> &'b O: IntoIterator<Item = &'b u8>,
        for<'b> &'b Os: IntoIterator<Item = &'b Opt<O>>
{
  fn get_size(&self) -> usize {
    let header_size = 4;
    let payload_marker_size = 1;
    let payload_size = self.payload.0.get_size();
    let token_size = self.token.0.len();
    let opts_size: usize = (&self.opts).into_iter().map(|o| o.get_size()).sum();

    header_size + payload_marker_size + payload_size + token_size + opts_size
  }

  fn max_size(&self) -> Option<usize> {
    None
  }
}

impl<C: Collection<u8>> GetSize for Opt<C> where for<'b> &'b C: IntoIterator<Item = &'b u8>
{
  fn get_size(&self) -> usize {
    let header_size = 1;
    let delta_size = match self.delta.0 {
      | n if n >= 269 => 2,
      | n if n >= 13 => 1,
      | _ => 0,
    };

    let value_len_size = match self.value.0.get_size() {
      | n if n >= 269 => 2,
      | n if n >= 13 => 1,
      | _ => 0,
    };

    header_size + delta_size + value_len_size + self.value.0.get_size()
  }

  fn max_size(&self) -> Option<usize> {
    None
  }
}
