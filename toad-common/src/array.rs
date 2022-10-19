use core::ops::{Deref, DerefMut};

/// Collections that can be mutably split in two
pub trait Split
  where Self: Sized
{
  /// Split a collection at some index, yielding 2 new collections
  /// where the first contains the range `0..n` and the second `n..`.
  ///
  /// _This is an abstracted [`Vec.split_off`]_
  ///
  /// ```
  /// use toad_common::Split;
  ///
  /// let vec = vec![1, 2, 3];
  ///
  /// let (before, after) = Split::split(vec, 1);
  /// assert_eq!(before, vec![1]);
  /// assert_eq!(after, vec![2, 3]);
  /// ```
  fn split(self, at: usize) -> (Self, Self);
}

macro_rules! veclike_split {
  ($self:ident, $at:ident) => {{
    let new = $self.split_off($at);
    ($self, new)
  }};
}

impl<T> Split for Vec<T> {
  fn split(mut self, at: usize) -> (Self, Self) {
    veclike_split!(self, at)
  }
}

impl<T, A: tinyvec::Array<Item = T>> Split for tinyvec::ArrayVec<A> {
  fn split(mut self, at: usize) -> (Self, Self) {
    veclike_split!(self, at)
  }
}

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

  /// Get the max size that this data structure can acommodate.
  ///
  /// By default, this returns `None` and can be left unimplemented for dynamic collections.
  ///
  /// However, for fixed-size collections this method must be implemented.
  ///
  /// ```
  /// use toad_common::GetSize;
  ///
  /// let stack_nums = tinyvec::ArrayVec::<[u8; 2]>::from([0, 1]);
  /// assert_eq!(stack_nums.max_size(), Some(2));
  /// ```
  fn max_size(&self) -> Option<usize>;

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

impl<T> GetSize for Vec<T> {
  fn get_size(&self) -> usize {
    self.len()
  }

  fn max_size(&self) -> Option<usize> {
    None
  }

  fn is_full(&self) -> bool {
    false
  }
}

impl<A: tinyvec::Array> GetSize for tinyvec::ArrayVec<A> {
  fn get_size(&self) -> usize {
    self.len()
  }

  fn max_size(&self) -> Option<usize> {
    Some(A::CAPACITY)
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

impl<T> Reserve for Vec<T> {
  fn reserve(n: usize) -> Self {
    Self::with_capacity(n)
  }
}

impl<A: tinyvec::Array> Reserve for tinyvec::ArrayVec<A> {}

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
  + Deref<Target = [<Self as Array>::Item]>
  + DerefMut
  + Extend<<Self as Array>::Item>
  + Split
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

impl<A: tinyvec::Array<Item = T>, T> Array for tinyvec::ArrayVec<A> {
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
