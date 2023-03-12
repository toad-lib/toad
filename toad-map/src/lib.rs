//! This microcrate contains a `Map` trait that generalizes `HashMap` semantics
//! to `std`, `alloc` and `no_std` platforms.

// docs
#![doc(html_root_url = "https://docs.rs/toad-map/0.0.0")]
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

use core::borrow::Borrow;
use core::hash::Hash;
use core::ops::{Deref, DerefMut};
use core::{iter, slice};
#[cfg(feature = "std")]
use std::collections::{hash_map, HashMap};

#[cfg(feature = "alloc")]
use std_alloc::collections::{btree_map, BTreeMap};
use toad_len::Len;

/// Things that can go unhappily when trying to insert into a map
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
pub enum InsertError<V> {
  /// The new value was inserted successfully but there was already a value in the map for that key.
  Exists(V),
  /// The map is at capacity and cannot fit any more pairs.
  CapacityExhausted,
}

/// An collection of key-value pairs
///
/// # Provided implementations
/// - [`HashMap`]`<K, V>`
/// - [`tinyvec::ArrayVec`]`<(K, V)>`
/// - [`Vec`]`<(K, V)>`
///
/// # Requirements
/// - [`Default`] for creating the map
/// - [`Extend`] for adding new entries to the map
/// - [`Len`] for bound checks, empty checks, and accessing the length
/// - [`FromIterator`] for [`collect`](core::iter::Iterator#method.collect)ing into the map
/// - [`IntoIterator`] for iterating and destroying the map
pub trait Map<K: Ord + Eq + Hash, V>:
  Default + Len + Extend<(K, V)> + FromIterator<(K, V)> + IntoIterator<Item = (K, V)>
{
  /// See [`HashMap.insert`]
  fn insert(&mut self, key: K, val: V) -> Result<(), InsertError<V>>;

  /// See [`HashMap.remove`]
  fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where K: Borrow<Q>,
          Q: Hash + Eq + Ord;

  /// See [`HashMap.get`]
  fn get<'a, Q: Hash + Eq + Ord>(&'a self, key: &Q) -> Option<&'a V>
    where K: Borrow<Q> + 'a;

  /// See [`HashMap.get_mut`]
  fn get_mut<'a, Q: Hash + Eq + Ord>(&'a mut self, key: &Q) -> Option<&'a mut V>
    where K: Borrow<Q> + 'a;

  /// See [`HashMap.contains_key`]
  fn has<Q: Hash + Eq + Ord>(&self, key: &Q) -> bool
    where K: Borrow<Q>
  {
    self.get(key).is_some()
  }

  /// See [`HashMap.iter`]
  fn iter(&self) -> Iter<'_, K, V>;

  /// See [`HashMap.iter_mut`]
  fn iter_mut(&mut self) -> IterMut<'_, K, V>;
}

#[cfg(feature = "alloc")]
impl<K: Eq + Hash + Ord, V> Map<K, V> for BTreeMap<K, V> {
  fn insert(&mut self, key: K, val: V) -> Result<(), InsertError<V>> {
    match self.insert(key, val).map(InsertError::Exists).ok_or(()) {
      | Ok(e) => Err(e),
      | Err(()) => Ok(()),
    }
  }

  fn remove<Q: Ord>(&mut self, key: &Q) -> Option<V>
    where K: Borrow<Q>
  {
    self.remove(key)
  }

  fn get<'a, Q: Hash + Eq + Ord>(&'a self, key: &Q) -> Option<&'a V>
    where K: Borrow<Q> + 'a
  {
    self.get(key)
  }

  fn get_mut<'a, Q: Hash + Eq + Ord>(&'a mut self, key: &Q) -> Option<&'a mut V>
    where K: Borrow<Q> + 'a
  {
    self.get_mut(key)
  }

  fn iter(&self) -> Iter<'_, K, V> {
    Iter { array_iter: None,
           #[cfg(feature = "std")]
           hashmap_iter: None,
           btreemap_iter: Some(self.iter()) }
  }

  fn iter_mut(&mut self) -> IterMut<'_, K, V> {
    IterMut { array_iter: None,
              #[cfg(feature = "std")]
              hashmap_iter: None,
              btreemap_iter: Some(self.iter_mut()) }
  }
}

#[cfg(feature = "std")]
impl<K: Eq + Hash + Ord, V> Map<K, V> for HashMap<K, V> {
  fn iter(&self) -> Iter<'_, K, V> {
    Iter { array_iter: None,
           btreemap_iter: None,
           hashmap_iter: Some(self.iter()) }
  }

  fn iter_mut(&mut self) -> IterMut<'_, K, V> {
    IterMut { array_iter: None,
              btreemap_iter: None,
              hashmap_iter: Some(self.iter_mut()) }
  }

  fn get<'a, Q: Hash + Eq + Ord>(&'a self, key: &Q) -> Option<&'a V>
    where K: Borrow<Q> + 'a
  {
    self.get(key)
  }

  fn get_mut<'a, Q: Hash + Eq + Ord>(&'a mut self, key: &Q) -> Option<&'a mut V>
    where K: Borrow<Q> + 'a
  {
    self.get_mut(key)
  }

  fn insert(&mut self, key: K, val: V) -> Result<(), InsertError<V>> {
    match self.insert(key, val).map(InsertError::Exists).ok_or(()) {
      | Ok(e) => Err(e),
      | Err(()) => Ok(()),
    }
  }

  fn remove<Q: Hash + Eq + Ord>(&mut self, key: &Q) -> Option<V>
    where K: Borrow<Q>
  {
    self.remove(key)
  }
}

impl<A: tinyvec::Array<Item = (K, V)>, K: Eq + Hash + Ord, V> Map<K, V> for tinyvec::ArrayVec<A> {
  fn insert(&mut self, key: K, mut val: V) -> Result<(), InsertError<V>> {
    match self.iter_mut().find(|(k, _)| k == &&key) {
      | Some((_, exist)) => {
        core::mem::swap(exist, &mut val);
        Err(InsertError::Exists(val))
      },
      | None => match self.is_full() {
        | true => Err(InsertError::CapacityExhausted),
        | false => {
          self.push((key, val));
          Ok(())
        },
      },
    }
  }

  fn remove<Q: Hash + Eq + Ord>(&mut self, key: &Q) -> Option<V>
    where K: Borrow<Q>
  {
    match self.iter()
              .enumerate()
              .find(|(_, (k, _))| Borrow::<Q>::borrow(*k) == key)
    {
      | Some((ix, _)) => Some(self.remove(ix).1),
      | None => None,
    }
  }

  fn get<'a, Q: Hash + Eq + Ord>(&'a self, key: &Q) -> Option<&'a V>
    where K: Borrow<Q> + 'a
  {
    match self.iter().find(|(k, _)| Borrow::<Q>::borrow(*k) == key) {
      | Some((_, v)) => Some(v),
      | None => None,
    }
  }

  fn get_mut<'a, Q: Hash + Eq + Ord>(&'a mut self, key: &Q) -> Option<&'a mut V>
    where K: Borrow<Q> + 'a
  {
    match self.iter_mut()
              .find(|(k, _)| Borrow::<Q>::borrow(*k) == key)
    {
      | Some((_, v)) => Some(v),
      | None => None,
    }
  }

  fn iter(&self) -> Iter<'_, K, V> {
    Iter { array_iter: Some(self.deref().iter().map(Iter::coerce_array_iter)),
           #[cfg(feature = "alloc")]
           btreemap_iter: None,
           #[cfg(feature = "std")]
           hashmap_iter: None }
  }

  fn iter_mut(&mut self) -> IterMut<'_, K, V> {
    IterMut { array_iter: Some(self.deref_mut().iter_mut().map(IterMut::coerce_array_iter)),
              #[cfg(feature = "alloc")]
              btreemap_iter: None,
              #[cfg(feature = "std")]
              hashmap_iter: None }
  }
}

#[cfg(feature = "alloc")]
impl<K, V> Map<K, V> for Vec<(K, V)> where K: Ord + Hash
{
  fn insert(&mut self, key: K, mut val: V) -> Result<(), InsertError<V>> {
    match self.iter_mut().find(|(k, _)| k == &&key) {
      | Some((_, exist)) => {
        core::mem::swap(exist, &mut val);
        Err(InsertError::Exists(val))
      },
      | None => match self.is_full() {
        | true => Err(InsertError::CapacityExhausted),
        | false => {
          self.push((key, val));
          Ok(())
        },
      },
    }
  }

  fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where K: Borrow<Q>,
          Q: Hash + Eq + Ord
  {
    match self.iter()
              .enumerate()
              .find(|(_, (k, _))| Borrow::<Q>::borrow(*k) == key)
    {
      | Some((ix, _)) => Some(self.remove(ix).1),
      | None => None,
    }
  }

  fn get<'a, Q: Hash + Eq + Ord>(&'a self, key: &Q) -> Option<&'a V>
    where K: Borrow<Q> + 'a
  {
    match self.iter().find(|(k, _)| Borrow::<Q>::borrow(*k) == key) {
      | Some((_, v)) => Some(v),
      | None => None,
    }
  }

  fn get_mut<'a, Q: Hash + Eq + Ord>(&'a mut self, key: &Q) -> Option<&'a mut V>
    where K: Borrow<Q> + 'a
  {
    match self.iter_mut()
              .find(|(k, _)| Borrow::<Q>::borrow(*k) == key)
    {
      | Some((_, v)) => Some(v),
      | None => None,
    }
  }

  fn iter(&self) -> Iter<'_, K, V> {
    Iter { array_iter: Some(self.deref().iter().map(Iter::coerce_array_iter)),
           #[cfg(feature = "alloc")]
           btreemap_iter: None,
           #[cfg(feature = "std")]
           hashmap_iter: None }
  }

  fn iter_mut(&mut self) -> IterMut<'_, K, V> {
    IterMut { array_iter: Some(self.deref_mut().iter_mut().map(IterMut::coerce_array_iter)),
              #[cfg(feature = "alloc")]
              btreemap_iter: None,
              #[cfg(feature = "std")]
              hashmap_iter: None }
  }
}

type ArrayIterCoercer<'a, K, V> = fn(&'a (K, V)) -> (&'a K, &'a V);
type ArrayIterMapped<'a, K, V> = iter::Map<slice::Iter<'a, (K, V)>, ArrayIterCoercer<'a, K, V>>;

type ArrayIterMutCoercer<'a, K, V> = fn(&'a mut (K, V)) -> (&'a K, &'a mut V);
type ArrayIterMutMapped<'a, K, V> =
  iter::Map<slice::IterMut<'a, (K, V)>, ArrayIterMutCoercer<'a, K, V>>;

/// An iterator over the entries of a `Map`.
///
/// This `struct` is created by the [`iter`] method on [`Map`].
/// See its documentation for more.
///
/// [`iter`]: Map::iter
///
/// # Example
///
/// ```
/// use std::collections::HashMap;
///
/// use toad_map::Map;
///
/// let mut map = HashMap::from([("a", 1)]);
///
/// fn do_stuff(map: &impl Map<&'static str, usize>) {
///   let iter = map.iter();
/// }
/// ```
#[derive(Debug)]
pub struct Iter<'a, K: Eq + Hash, V> {
  #[cfg(feature = "std")]
  hashmap_iter: Option<hash_map::Iter<'a, K, V>>,
  #[cfg(feature = "alloc")]
  btreemap_iter: Option<btree_map::Iter<'a, K, V>>,
  array_iter: Option<ArrayIterMapped<'a, K, V>>,
}

impl<'a, K: Eq + Hash, V> Iter<'a, K, V> {
  #[inline(always)]
  fn coerce_array_iter((k, v): &'a (K, V)) -> (&'a K, &'a V) {
    (k, v)
  }

  #[allow(unreachable_code)]
  fn get_iter(&mut self) -> &mut dyn Iterator<Item = (&'a K, &'a V)> {
    #[cfg(feature = "std")]
    {
      let (a, b, c) = (self.hashmap_iter.as_mut().map(|a| a as &mut _),
                       self.array_iter.as_mut().map(|a| a as &mut _),
                       self.btreemap_iter.as_mut().map(|a| a as &mut _));
      return a.or(b).or(c).unwrap();
    };

    #[cfg(feature = "alloc")]
    {
      let (a, b) = (self.array_iter.as_mut().map(|a| a as &mut _),
                    self.btreemap_iter.as_mut().map(|a| a as &mut _));
      return a.or(b).unwrap();
    }

    // no_std and no alloc; must be array
    self.array_iter.as_mut().map(|a| a as &mut _).unwrap()
  }
}

impl<'a, K: Eq + Hash, V> Iterator for Iter<'a, K, V> {
  type Item = (&'a K, &'a V);

  fn next(&mut self) -> Option<Self::Item> {
    self.get_iter().next()
  }
}

/// A mutable iterator over the entries of a `Map`.
///
/// This `struct` is created by the [`iter_mut`] method on [`Map`]. See its
/// documentation for more.
///
/// [`iter_mut`]: Map::iter_mut
///
/// # Example
///
/// ```
/// use std::collections::HashMap;
///
/// use toad_map::Map;
///
/// let mut map = HashMap::from([("a", 1)]);
///
/// fn do_stuff(map: &mut impl Map<&'static str, usize>) {
///   let iter = map.iter_mut();
/// }
/// ```
#[derive(Debug)]
pub struct IterMut<'a, K: Eq + Hash, V> {
  #[cfg(feature = "std")]
  hashmap_iter: Option<hash_map::IterMut<'a, K, V>>,
  #[cfg(feature = "alloc")]
  btreemap_iter: Option<btree_map::IterMut<'a, K, V>>,
  array_iter: Option<ArrayIterMutMapped<'a, K, V>>,
}

impl<'a, K: Eq + Hash, V> IterMut<'a, K, V> {
  #[inline(always)]
  fn coerce_array_iter((k, v): &'a mut (K, V)) -> (&'a K, &'a mut V) {
    (k, v)
  }

  #[allow(unreachable_code)]
  fn get_iter(&mut self) -> &mut dyn Iterator<Item = (&'a K, &'a mut V)> {
    #[cfg(feature = "std")]
    {
      let (a, b, c) = (self.hashmap_iter.as_mut().map(|a| a as &mut _),
                       self.array_iter.as_mut().map(|a| a as &mut _),
                       self.btreemap_iter.as_mut().map(|a| a as &mut _));
      return a.or(b).or(c).unwrap();
    };

    #[cfg(feature = "alloc")]
    {
      let (a, b) = (self.array_iter.as_mut().map(|a| a as &mut _),
                    self.btreemap_iter.as_mut().map(|a| a as &mut _));
      return a.or(b).unwrap();
    }

    // no_std and no alloc; must be array
    self.array_iter.as_mut().map(|a| a as &mut _).unwrap()
  }
}

impl<'a, K: Eq + Hash, V> Iterator for IterMut<'a, K, V> {
  type Item = (&'a K, &'a mut V);

  fn next(&mut self) -> Option<Self::Item> {
    self.get_iter().next()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn impls(
    )
      -> (impl Map<String, String>,
          impl Map<String, String>,
          impl Map<String, String>,
          impl Map<String, String>)
  {
    (HashMap::<String, String>::from([("foo".into(), "bar".into())]),
     BTreeMap::<String, String>::from([("foo".into(), "bar".into())]),
     tinyvec::array_vec!([(String, String); 16] => ("foo".into(), "bar".into())),
     vec![("foo".to_string(), "bar".to_string())])
  }

  macro_rules! each_impl {
    ($work:expr) => {{
      let (hm, bt, av, vc) = impls();
      println!("hashmap");
      $work(hm);
      println!("btreemap");
      $work(bt);
      println!("arrayvec");
      $work(av);
      println!("vec");
      $work(vc);
    }};
  }

  #[test]
  fn get() {
    fn test_get<M: Map<String, String>>(map: M) {
      assert_eq!(map.get(&"foo".to_string()), Some(&"bar".into()));
      assert_eq!(map.get(&"foot".to_string()), None);
    }

    each_impl!(test_get);
  }

  #[test]
  fn get_mut() {
    fn test_get_mut<M: Map<String, String>>(mut map: M) {
      let old = map.get_mut(&"foo".to_string()).unwrap();
      *old = format!("{}f", old);

      assert_eq!(map.get(&"foo".to_string()), Some(&"barf".into()));
    }

    each_impl!(test_get_mut);
  }

  #[test]
  fn insert() {
    fn test_insert<M: Map<String, String>>(mut map: M) {
      let old = map.insert("foot".to_string(), "butt".to_string());

      assert_eq!(old, Ok(()));
      assert_eq!(map.get(&"foo".to_string()).unwrap().as_str(), "bar");
      assert_eq!(map.get(&"foot".to_string()).unwrap().as_str(), "butt");

      let old = map.insert("foot".to_string(), "squat".to_string());
      assert_eq!(old, Err(InsertError::Exists("butt".to_string())));
      assert_eq!(map.get(&"foot".to_string()).unwrap().as_str(), "squat");
    }

    each_impl!(test_insert);
  }

  #[test]
  fn remove() {
    fn test_remove<M: Map<String, String>>(mut map: M) {
      let old = map.remove(&"foo".to_string());
      assert_eq!(old, Some("bar".to_string()));

      let old = map.remove(&"foo".to_string());
      assert_eq!(old, None);
    }

    each_impl!(test_remove);
  }

  #[test]
  fn has() {
    fn test_has<M: Map<String, String>>(map: M) {
      assert!(map.has(&"foo".to_string()));
      assert!(!map.has(&"foot".to_string()));
    }

    each_impl!(test_has);
  }

  #[test]
  fn into_iter() {
    fn test_into_iter<M: Map<String, String>>(mut map: M) {
      map.insert("a".into(), "a".into()).unwrap();
      map.insert("b".into(), "b".into()).unwrap();
      map.insert("c".into(), "c".into()).unwrap();

      let mut kvs = map.into_iter().collect::<Vec<_>>();
      kvs.sort();

      assert_eq!(kvs,
                 vec![("a".into(), "a".into()),
                      ("b".into(), "b".into()),
                      ("c".into(), "c".into()),
                      ("foo".into(), "bar".into()),]);
    }

    each_impl!(test_into_iter);
  }

  #[test]
  fn iter() {
    fn test_iter<M: Map<String, String>>(mut map: M) {
      map.insert("a".into(), "a".into()).unwrap();
      map.insert("b".into(), "b".into()).unwrap();
      map.insert("c".into(), "c".into()).unwrap();

      let mut kvs = map.iter().collect::<Vec<_>>();
      kvs.sort();

      assert_eq!(kvs,
                 vec![(&"a".into(), &"a".into()),
                      (&"b".into(), &"b".into()),
                      (&"c".into(), &"c".into()),
                      (&"foo".into(), &"bar".into()),]);
    }

    each_impl!(test_iter);
  }

  #[test]
  fn iter_mut() {
    fn test_iter_mut<M: Map<String, String>>(mut map: M) {
      map.insert("a".into(), "a".into()).unwrap();
      map.insert("b".into(), "b".into()).unwrap();
      map.insert("c".into(), "c".into()).unwrap();

      let mut kvs = map.iter_mut().collect::<Vec<_>>();
      kvs.sort();

      assert_eq!(kvs,
                 vec![(&"a".into(), &mut "a".into()),
                      (&"b".into(), &mut "b".into()),
                      (&"c".into(), &mut "c".into()),
                      (&"foo".into(), &mut "bar".into()),]);
    }

    each_impl!(test_iter_mut);
  }
}
