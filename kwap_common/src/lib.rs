//! Common structs and abstractions used by `kwap`

#![doc(html_root_url = "https://docs.rs/kwap-common/0.3.0")]
#![cfg_attr(all(not(test), feature = "no_std"), no_std)]
#![cfg_attr(not(test), forbid(missing_debug_implementations, unreachable_pub))]
#![cfg_attr(not(test), deny(unsafe_code, missing_copy_implementations))]
#![cfg_attr(any(docsrs, feature = "docs"), feature(doc_cfg))]
#![deny(missing_docs)]

extern crate alloc;
use alloc::vec::Vec;

/// An ordered collection of some type `T`.
///
/// # Provided implementations
/// - [`Vec`]
/// - [`tinyvec::ArrayVec`]
///
/// Notably, not `heapless::ArrayVec` or `arrayvec::ArrayVec`. An important usecase
/// is [`Extend`]ing the collection, and the performance of `heapless` and `arrayvec`'s Extend implementations
/// are notably worse than `tinyvec`.
///
/// `tinyvec` also has the added bonus of being 100% unsafe-code-free, meaning if you choose `tinyvec` you eliminate the
/// possibility of memory defects and UB.
///
/// # Requirements
/// - `Default` for creating the collection
/// - `Extend` for mutating and adding onto the collection (1 or more elements)
/// - `Reserve` for reserving space ahead of time
/// - `GetSize` for bound checks, empty checks, and accessing the length
/// - `FromIterator` for collecting into the collection
/// - `IntoIterator` for:
///    - iterating and destroying the collection
///    - for iterating over references to items in the collection
///
/// # Stupid `where` clause
/// `where for<'a> &'a Self: IntoIterator<Item = &'a T>` is necessary to fold in the idea
/// of "A reference (of any arbitrary lifetime `'a`) to an Array must support iterating over references (`'a`) of its elements."
///
/// A side-effect of this where clause is that because it's not a trait bound, it must be propagated to every bound that requires an `Array`.
///
/// Less than ideal, but far preferable to coupling tightly to a particular collection and maintaining separate `alloc` and non-`alloc` implementations.
pub trait Array<T>:
  Default + Insert<T> + GetSize + Reserve + Extend<T> + FromIterator<T> + IntoIterator<Item = T>
  where for<'a> &'a Self: IntoIterator<Item = &'a T>
{
}

impl<T> Array<T> for Vec<T> {}
impl<A: tinyvec::Array<Item = T>, T> Array<T> for tinyvec::ArrayVec<A> {}

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
  /// use kwap_common::GetSize;
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
  /// use kwap_common::GetSize;
  ///
  /// assert!(Vec::<u8>::new().size_is_zero())
  /// ```
  fn size_is_zero(&self) -> bool {
    self.get_size() == 0
  }

  /// Is there no room left in this collection?
  ///
  /// ```
  /// use kwap_common::GetSize;
  ///
  /// let array = tinyvec::ArrayVec::<[u8; 2]>::from([1, 2]);
  ///
  /// assert!(array.is_full())
  /// ```
  fn is_full(&self) -> bool {
    self.max_size().map(|max| self.get_size() >= max).unwrap_or(false)
  }
}

impl<T> GetSize for Vec<T> {
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

/// Create a data structure and reserve some amount of space for it to grow into
///
/// # Examples
/// - `Vec` is `Reserve`, and invokes `Vec::with_capacity`
/// - `tinyvec::ArrayVec` is `Reserve` and invokes `Default::default()` because creating an `ArrayVec` automatically allocates the required space on the stack.
pub trait Reserve: Default {
  /// Create an instance of the collection with a given capacity.
  ///
  /// Used to reserve some contiguous space, e.g. [`Vec::with_capacity`]
  ///
  /// The default implementation invokes `Default::default`
  fn reserve(_: usize) -> Self {
    Default::default()
  }
}

impl<T> Reserve for Vec<T> {
  fn reserve(n: usize) -> Self {
    Self::with_capacity(n)
  }
}

impl<A: tinyvec::Array> Reserve for tinyvec::ArrayVec<A> {}

/// Insert items into collections
///
/// ```
/// use kwap_common::Insert;
///
/// let mut nums = vec![1, 2, 3];
/// Insert::push(&mut nums, 4);
/// assert_eq!(nums, vec![1, 2, 3, 4]);
///
/// nums.insert_at(0, 0);
/// assert_eq!(nums, vec![0, 1, 2, 3, 4]);
/// ```
pub trait Insert<T>: GetSize {
  /// Insert a value at a particular index of a collection.
  fn insert_at(&mut self, index: usize, value: T);

  /// Add a value to the end of a collection.
  fn push(&mut self, value: T) {
    self.insert_at(self.get_size(), value)
  }
}

impl<T> Insert<T> for Vec<T> {
  fn insert_at(&mut self, index: usize, value: T) {
    self.insert(index, value);
  }

  fn push(&mut self, value: T) {
    self.push(value)
  }
}

impl<A: tinyvec::Array> Insert<A::Item> for tinyvec::ArrayVec<A> {
  fn insert_at(&mut self, index: usize, value: A::Item) {
    self.insert(index, value);
  }

  fn push(&mut self, value: A::Item) {
    self.push(value)
  }
}
