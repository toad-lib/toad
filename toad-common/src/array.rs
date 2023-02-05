use core::ops::{Deref, DerefMut};

#[cfg(feature = "alloc")]
use std_alloc::vec::Vec;

/// Get the runtime size of some data structure
///
/// # Deprecated
/// Note: in a future version of `toad_common` this will be deprecated in favor of clearly delineating
/// "size in bytes" (e.g. `RuntimeSize`) from "collection of potentially bounded length" (e.g. `Len`)
///
/// ## Collections
/// For collections this just yields the number of elements ([`Vec::len`], [`tinyvec::ArrayVec::len`]),
/// and when the collection is over [`u8`]s,
/// then `get_size` represents the number of bytes in the collection.
///
/// ## Structs and enums
/// When implemented for items that are not collections,
/// this is expected to yield the runtime size in bytes
/// (not the static Rust [`core::mem::size_of`] size)
pub trait GetSize {
  /// Get the max size that this data structure can acommodate.
  const CAPACITY: Option<usize>;

  /// Get the runtime size (in bytes) of a struct
  ///
  /// For collections this is always equivalent to calling an inherent `len` method.
  ///
  /// ```
  /// use toad_common::GetSize;
  ///
  /// assert_eq!(vec![1u8, 2].get_size(), 2)
  /// ```
  fn get_size(&self) -> usize;

  /// Check if the runtime size is zero
  ///
  /// ```
  /// use toad_common::GetSize;
  ///
  /// assert!(Vec::<u8>::new().size_is_zero())
  /// ```
  fn size_is_zero(&self) -> bool {
    self.get_size() == 0
  }

  /// Is there no room left in this collection?
  ///
  /// ```
  /// use toad_common::GetSize;
  ///
  /// let array = tinyvec::ArrayVec::<[u8; 2]>::from([1, 2]);
  ///
  /// assert!(array.is_full())
  /// ```
  fn is_full(&self) -> bool;
}

#[cfg(feature = "alloc")]
impl<T> GetSize for Vec<T> {
  const CAPACITY: Option<usize> = None;

  fn get_size(&self) -> usize {
    self.len()
  }

  fn is_full(&self) -> bool {
    false
  }
}

impl<A: tinyvec::Array> GetSize for tinyvec::ArrayVec<A> {
  const CAPACITY: Option<usize> = Some(A::CAPACITY);

  fn get_size(&self) -> usize {
    self.len()
  }

  fn is_full(&self) -> bool {
    self.len() >= self.capacity()
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

/// Truncate this collection to a new length.
///
/// If self was shorter than `len`, nothing happens.
///
/// If self was longer, drops elements up to `len`
pub trait Trunc
  where Self: Sized
{
  #[allow(missing_docs)]
  fn trunc(&mut self, len: usize) -> ();

  /// Erase all elements in the collection
  fn clear(&mut self) {
    self.trunc(0);
  }
}

#[cfg(feature = "alloc")]
impl<T> Trunc for Vec<T> {
  fn trunc(&mut self, len: usize) -> () {
    self.truncate(len)
  }
}

impl<T, const N: usize> Trunc for tinyvec::ArrayVec<[T; N]> where T: Default
{
  fn trunc(&mut self, len: usize) -> () {
    self.truncate(len)
  }
}

/// Fill this collection to the end with copies of `t`,
/// copying array initialization `[0u8; 1000]` to the [`Array`] trait.
///
/// If the collection has no end (e.g. [`Vec`]),
/// this trait's methods will return `None`.
pub trait Filled<T>: Sized {
  #[allow(missing_docs)]
  fn filled(t: T) -> Option<Self>
    where T: Copy
  {
    Self::filled_using(|| t)
  }

  #[allow(missing_docs)]
  fn filled_default() -> Option<Self>
    where T: Default
  {
    Self::filled_using(|| Default::default())
  }

  #[allow(missing_docs)]
  fn filled_using<F>(f: F) -> Option<Self>
    where F: Fn() -> T;
}

#[cfg(feature = "alloc")]
impl<T> Reserve for Vec<T> {
  fn reserve(n: usize) -> Self {
    Self::with_capacity(n)
  }
}

#[cfg(feature = "alloc")]
impl<T> Filled<T> for Vec<T> {
  fn filled_using<F>(_: F) -> Option<Self>
    where F: Fn() -> T
  {
    None
  }
}

impl<A: tinyvec::Array> Reserve for tinyvec::ArrayVec<A> {}

impl<T, const N: usize> Filled<T> for tinyvec::ArrayVec<[T; N]> where T: Default
{
  fn filled_using<F>(f: F) -> Option<Self>
    where F: Fn() -> T
  {
    Some(core::iter::repeat(()).take(N).map(|_| f()).collect())
  }

  fn filled(t: T) -> Option<Self>
    where T: Copy
  {
    Some(Self::from([t; N]))
  }
}

/// An ordered indexable collection of some type `Item`
///
/// # Provided implementations
/// - [`Vec`]
/// - [`tinyvec::ArrayVec`]
///
/// Notably, not `heapless::ArrayVec` or `arrayvec::ArrayVec`. An important usecase within `toad`
/// is [`Extend`]ing the collection, and the performance of `heapless` and `arrayvec`'s Extend implementations
/// are notably worse than `tinyvec`.
///
/// `tinyvec` also has the added bonus of being 100% unsafe-code-free, meaning if you choose `tinyvec` you eliminate the
/// possibility of memory defects and UB.
///
/// # Requirements
/// - [`Default`] for creating the collection
/// - [`Extend`] for mutating and adding onto the collection (1 or more elements)
/// - [`Reserve`] for reserving space ahead of time
/// - [`GetSize`] for bound checks, empty checks, and accessing the length
/// - [`FromIterator`] for [`collect`](core::iter::Iterator#method.collect)ing into the collection
/// - [`IntoIterator`] for iterating and destroying the collection
/// - [`Deref<Target = [T]>`](Deref) and [`DerefMut`] for:
///    - indexing ([`Index`](core::ops::Index), [`IndexMut`](core::ops::IndexMut))
///    - iterating ([`&[T].iter()`](primitive@slice#method.iter) and [`&mut [T].iter_mut()`](primitive@slice#method.iter_mut))
pub trait Array:
  Default
  + GetSize
  + Reserve
  + Filled<<Self as Array>::Item>
  + Trunc
  + Deref<Target = [<Self as Array>::Item]>
  + DerefMut
  + Extend<<Self as Array>::Item>
  + FromIterator<<Self as Array>::Item>
  + IntoIterator<Item = <Self as Array>::Item>
{
  /// The type of item contained in the collection
  type Item;

  /// Insert a value at a particular index of a collection.
  fn insert_at(&mut self, index: usize, value: <Self as Array>::Item);

  /// Try to remove an entry from the collection.
  ///
  /// Returns `Some(Self::Item)` if `index` was in-bounds, `None` if `index` is out of bounds.
  fn remove(&mut self, index: usize) -> Option<<Self as Array>::Item>;

  /// Add a value to the end of a collection.
  fn push(&mut self, value: <Self as Array>::Item);
}

/// Collections that support extending themselves mutably from copyable slices
pub trait AppendCopy<T: Copy> {
  /// Extend self mutably, copying from a slice.
  ///
  /// Worst-case implementations copy 1 element at a time (time O(n))
  ///
  /// Best-case implementations copy as much of the origin slice
  /// at once as possible (system word size), e.g. [`Vec::append`].
  /// (still linear time, but on 64-bit systems this is 64 times faster than a 1-by-1 copy.)
  fn append_copy(&mut self, i: &[T]);
}

#[cfg(feature = "alloc")]
impl<T: Copy> AppendCopy<T> for Vec<T> {
  fn append_copy(&mut self, i: &[T]) {
    self.extend(i);
  }
}

impl<T: Copy, A: tinyvec::Array<Item = T>> AppendCopy<T> for tinyvec::ArrayVec<A> {
  fn append_copy(&mut self, i: &[T]) {
    self.extend_from_slice(i);
  }
}

#[cfg(feature = "alloc")]
impl<T> Array for Vec<T> {
  type Item = T;

  fn insert_at(&mut self, index: usize, value: T) {
    self.insert(index, value);
  }

  fn remove(&mut self, index: usize) -> Option<T> {
    if index < self.len() {
      Some(Vec::remove(self, index))
    } else {
      None
    }
  }

  fn push(&mut self, value: T) {
    self.push(value)
  }
}

impl<A: tinyvec::Array<Item = T>, T> Array for tinyvec::ArrayVec<A> where Self: Filled<T> + Trunc
{
  type Item = T;

  fn insert_at(&mut self, index: usize, value: A::Item) {
    self.insert(index, value);
  }

  fn remove(&mut self, index: usize) -> Option<T> {
    if index < self.len() {
      Some(tinyvec::ArrayVec::remove(self, index))
    } else {
      None
    }
  }

  fn push(&mut self, value: A::Item) {
    self.push(value)
  }
}
