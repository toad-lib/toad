//! This microcrate contains the `Array` trait used by the
//! [`toad`](https://github.com/toad-lib/toad) CoAP runtime / ecosystem.
//!
//! The `Array` trait defines common operations used with heap-allocated
//! collections like [`Vec`](https://doc.rust-lang.org/std/vec/struct.Vec.html) but
//! is also implemented for [`tinyvec::ArrayVec`](https://docs.rs/tinyvec/latest) allowing
//! for applications to be usable on platforms with or without heap allocation.

// docs
#![doc(html_root_url = "https://docs.rs/toad-array/0.1.0")]
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

use core::ops::{Deref, DerefMut};

#[cfg(feature = "alloc")]
use std_alloc::vec::Vec;
use toad_len::Len;

/// Operations on ordered indexed collections
pub trait Indexed<T>
  where Self: Len + Deref<Target = [T]>
{
  /// Insert a new element at `ix`, shifting all other elements to the right.
  ///
  /// ```
  /// use toad_array::Indexed;
  ///
  /// fn do_stuff<I: Indexed<u32> + AsRef<Vec<u32>>>(mut i: I) {
  ///   i.insert(0, 2);
  ///   assert_eq!(i.as_ref(), &vec![2]);
  ///
  ///   i.insert(0, 1);
  ///   assert_eq!(i.as_ref(), &vec![1, 2]);
  ///
  ///   i.insert(2, 3);
  ///   assert_eq!(i.as_ref(), &vec![1, 2, 3]);
  /// }
  ///
  /// do_stuff(vec![]);
  /// ```
  fn insert(&mut self, ix: usize, t: T);

  /// Remove element at `ix`, shifting all other elements to the left.
  ///
  /// ```
  /// use toad_array::Indexed;
  ///
  /// fn do_stuff<I: Indexed<u32> + AsRef<Vec<u32>>>(mut i: I) {
  ///   i.remove(1);
  ///   assert_eq!(i.as_ref(), &vec![1]);
  ///
  ///   i.remove(0);
  ///   assert_eq!(i.as_ref(), &vec![]);
  ///
  ///   i.remove(0);
  ///   assert_eq!(i.as_ref(), &vec![]);
  /// }
  ///
  /// do_stuff(vec![1, 2]);
  /// ```
  fn remove(&mut self, ix: usize) -> Option<T>;

  /// Insert an element at the front of the collection
  ///
  /// ```
  /// use toad_array::Indexed;
  ///
  /// fn do_stuff<I: Indexed<u32> + AsRef<Vec<u32>>>(mut i: I) {
  ///   i.push(3);
  ///   assert_eq!(i.as_ref(), &vec![3]);
  ///
  ///   i.push(2);
  ///   assert_eq!(i.as_ref(), &vec![2, 3]);
  ///
  ///   i.push(1);
  ///   assert_eq!(i.as_ref(), &vec![1, 2, 3]);
  /// }
  ///
  /// do_stuff(vec![]);
  /// ```
  fn push(&mut self, t: T) {
    self.insert(0, t)
  }

  /// Insert an element at the end of the collection
  ///
  /// ```
  /// use toad_array::Indexed;
  ///
  /// fn do_stuff<I: Indexed<u32> + AsRef<Vec<u32>>>(mut i: I) {
  ///   i.append(3);
  ///   assert_eq!(i.as_ref(), &vec![3]);
  ///
  ///   i.append(2);
  ///   assert_eq!(i.as_ref(), &vec![3, 2]);
  ///
  ///   i.append(1);
  ///   assert_eq!(i.as_ref(), &vec![3, 2, 1]);
  /// }
  ///
  /// do_stuff(vec![]);
  /// ```
  fn append(&mut self, t: T) {
    self.insert(self.len(), t)
  }

  /// Drop `ct` elements from the front of the collection
  ///
  /// ```
  /// use toad_array::Indexed;
  ///
  /// let mut v: Vec<u32> = vec![1, 2, 3, 4];
  ///
  /// v.drop_front(2);
  /// assert_eq!(v, vec![3, 4]);
  ///
  /// v.drop_front(3);
  /// assert_eq!(v, vec![]);
  ///
  /// v.drop_front(1);
  /// assert_eq!(v, vec![]);
  /// ```
  fn drop_front(&mut self, ct: usize) {
    if !self.is_empty() && ct > 0 {
      self.remove(0);
      self.drop_front(ct - 1);
    }
  }

  /// Drop `ct` elements from the back of the collection
  ///
  /// ```
  /// use toad_array::Indexed;
  ///
  /// let mut v: Vec<u32> = vec![1, 2, 3, 4];
  ///
  /// v.drop_back(2);
  /// assert_eq!(v, vec![1, 2]);
  ///
  /// v.drop_back(2);
  /// assert_eq!(v, vec![]);
  ///
  /// v.drop_back(1);
  /// assert_eq!(v, vec![]);
  /// ```
  fn drop_back(&mut self, ct: usize) {
    if !self.is_empty() && ct > 0 {
      self.remove(self.len() - 1);
      self.drop_back(ct - 1);
    }
  }

  /// Drop elements from the front of the collection until
  /// the collection is emptied or the predicate returns
  /// false.
  ///
  /// ```
  /// use toad_array::Indexed;
  ///
  /// let mut v: Vec<u32> = vec![2, 4, 6, 5];
  ///
  /// v.drop_while(|n| n % 2 == 0);
  /// assert_eq!(v, vec![5]);
  /// ```
  fn drop_while<F>(&mut self, f: F)
    where F: for<'a> Fn(&'a T) -> bool
  {
    match self.get(0) {
      | Some(t) if !f(&t) => return,
      | None => return,
      | _ => (),
    };

    self.remove(0);
    self.drop_while(f);
  }
}

/// Create a data structure and reserve some amount of space for it to grow into
///
/// # Examples
/// - `Vec` is `Reserve`, and invokes `Vec::with_capacity`
/// - `tinyvec::ArrayVec` is `Reserve` and invokes `Default::default()` because creating an `ArrayVec` automatically allocates the required space on the stack.
pub trait Reserve
  where Self: Default
{
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

/// A generalization of [`std::vec::Vec`]
///
/// # Provided implementations
/// - [`Vec`]
/// - [`tinyvec::ArrayVec`]
///
/// ## Why [`tinyvec::ArrayVec`]?
/// The performance of `heapless` and `arrayvec`'s Extend implementations
/// are notably worse than `tinyvec`. (see `toad-msg/benches/collections.rs`)
/// `tinyvec` also has the added bonus of being 100% unsafe-code-free.
///
/// # Definition of an [`Array`]
/// The Array trait is automatically implemented for ordered indexed collections
/// with a non-fixed number of elements which are contiguous in memory.
///
/// This translates to the trait requirements:
/// - Must have an empty ([`Default`]) value
/// - Must allow populating every element with a value ([`Filled`])
/// - Must allow dropping every element after a given index ([`Trunc`])
/// - Must allow mutably appending one or more elements ([`Extend`])
/// - Must be creatable from an iterator ([`FromIterator`])
/// - Must allow iterating over owned elements ([`IntoIterator`])
/// - Must be dereferenceable to readonly and mutable slices ([`Deref`], [`DerefMut`])
/// - Must allow getting the runtime length ([`Len`])
/// - May have a hard limit on number of elements ([`Len`])
/// - May allow creating an instance with maximum length and a given filler value ([`Filled`])
/// - May allow pre-allocating space for a specific number of elements ([`Reserve`])
pub trait Array:
  Default
  + Len
  + Reserve
  + Filled<<Self as Array>::Item>
  + Trunc
  + Indexed<<Self as Array>::Item>
  + Extend<<Self as Array>::Item>
  + FromIterator<<Self as Array>::Item>
  + IntoIterator<Item = <Self as Array>::Item>
  + Deref<Target = [<Self as Array>::Item]>
  + DerefMut
{
  /// The type of item contained in the collection
  type Item;
}

/// Collections that support extending themselves mutably from copyable slices
pub trait AppendCopy<T>
  where T: Copy
{
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
impl<T> AppendCopy<T> for Vec<T> where T: Copy
{
  fn append_copy(&mut self, i: &[T]) {
    self.extend(i);
  }
}

impl<T, A> AppendCopy<T> for tinyvec::ArrayVec<A>
  where T: Copy,
        A: tinyvec::Array<Item = T>
{
  fn append_copy(&mut self, i: &[T]) {
    self.extend_from_slice(i);
  }
}

#[cfg(feature = "alloc")]
impl<T> Array for Vec<T> {
  type Item = T;
}

#[cfg(feature = "alloc")]
impl<T> Indexed<T> for Vec<T> {
  fn insert(&mut self, index: usize, value: T) {
    self.insert(index, value);
  }

  fn remove(&mut self, index: usize) -> Option<T> {
    if index < self.len() {
      Some(Vec::remove(self, index))
    } else {
      None
    }
  }
}

impl<A, T> Array for tinyvec::ArrayVec<A>
  where Self: Filled<T> + Trunc,
        A: tinyvec::Array<Item = T>
{
  type Item = T;
}

impl<A> Indexed<A::Item> for tinyvec::ArrayVec<A>
  where Self: Filled<A::Item> + Trunc,
        A: tinyvec::Array
{
  fn insert(&mut self, ix: usize, t: A::Item) {
    tinyvec::ArrayVec::insert(self, ix, t)
  }

  fn remove(&mut self, ix: usize) -> Option<A::Item> {
    if ix < self.len() {
      Some(tinyvec::ArrayVec::remove(self, ix))
    } else {
      None
    }
  }
}
