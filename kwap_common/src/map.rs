use core::borrow::Borrow;
use std::collections::{hash_map, HashMap};
use std::hash::Hash;
use std::{iter, slice};

use crate::result::ResultExt;
use crate::{GetSize, Reserve};

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
/// - [`Reserve`] for reserving space ahead of time
/// - [`GetSize`] for bound checks, empty checks, and accessing the length
/// - [`FromIterator`] for [`collect`](core::iter::Iterator#method.collect)ing into the map
/// - [`IntoIterator`] for iterating and destroying the map
pub trait Map<K: Eq + Hash, V>:
  Default + GetSize + Reserve + Extend<(K, V)> + FromIterator<(K, V)> + IntoIterator<Item = (K, V)>
{
  /// See [`HashMap.insert`]
  fn insert(&mut self, key: K, val: V) -> Result<(), InsertError<V>>;

  /// See [`HashMap.remove`]
  fn remove<Q: Hash + Eq>(&mut self, key: &Q) -> Option<V>
    where K: Borrow<Q>;

  /// See [`HashMap.get`]
  fn get<'a, Q: Hash + Eq>(&'a self, key: &Q) -> Option<&'a V>
    where K: Borrow<Q> + 'a;

  /// See [`HashMap.get_mut`]
  fn get_mut<'a, Q: Hash + Eq>(&'a mut self, key: &Q) -> Option<&'a mut V>
    where K: Borrow<Q> + 'a;

  /// See [`HashMap.contains_key`]
  fn has<Q: Hash + Eq>(&self, key: &Q) -> bool
    where K: Borrow<Q>
  {
    self.get(key).is_some()
  }

  /// See [`HashMap.iter`]
  fn iter<'a>(&'a self) -> Iter<'a, K, V>;

  /// See [`HashMap.iter_mut`]
  fn iter_mut<'a>(&'a mut self) -> IterMut<'a, K, V>;
}

impl<K: Eq + Hash, V> Map<K, V> for HashMap<K, V> {
  fn iter<'a>(&'a self) -> Iter<'a, K, V> {
    Iter { array_iter: None,
           hashmap_iter: Some(self.iter()) }
  }

  fn iter_mut<'a>(&'a mut self) -> IterMut<'a, K, V> {
    IterMut { array_iter: None,
              hashmap_iter: Some(self.iter_mut()) }
  }

  fn get<'a, Q: Hash + Eq>(&'a self, key: &Q) -> Option<&'a V>
    where K: Borrow<Q> + 'a
  {
    self.get(&key)
  }

  fn get_mut<'a, Q: Hash + Eq>(&'a mut self, key: &Q) -> Option<&'a mut V>
    where K: Borrow<Q> + 'a
  {
    self.get_mut(key)
  }

  fn insert(&mut self, key: K, val: V) -> Result<(), InsertError<V>> {
    self.insert(key, val).map(InsertError::Exists).ok_or(()).swap()
  }

  fn remove<Q: Hash + Eq>(&mut self, key: &Q) -> Option<V>
    where K: Borrow<Q>
  {
    self.remove(key)
  }
}

impl<T: crate::Array<Item = (K, V)>, K: Eq + Hash, V> Map<K, V> for T {
  fn insert(&mut self, key: K, mut val: V) -> Result<(), InsertError<V>> {
    match self.iter_mut().find(|(k, _)| k == &&key) {
      | Some((_, exist)) => {
        std::mem::swap(exist, &mut val);
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

  fn remove<Q: Hash + Eq>(&mut self, key: &Q) -> Option<V>
    where K: Borrow<Q>
  {
    match self.iter()
              .enumerate()
              .find(|(_, (k, _))| Borrow::<Q>::borrow(*k) == key)
    {
      | Some((ix, _)) => self.remove(ix).map(|(_, v)| v),
      | None => None,
    }
  }

  fn get<'a, Q: Hash + Eq>(&'a self, key: &Q) -> Option<&'a V>
    where K: Borrow<Q> + 'a
  {
    match self.iter().find(|(k, _)| Borrow::<Q>::borrow(*k) == key) {
      | Some((_, ref v)) => Some(v),
      | None => None,
    }
  }

  fn get_mut<'a, Q: Hash + Eq>(&'a mut self, key: &Q) -> Option<&'a mut V>
    where K: Borrow<Q> + 'a
  {
    match self.iter_mut().find(|(k, _)| Borrow::<Q>::borrow(*k) == key) {
      | Some((_, v)) => Some(v),
      | None => None,
    }
  }

  fn iter<'a>(&'a self) -> Iter<'a, K, V> {
    Iter { array_iter: Some(self.deref().iter().map(Iter::coerce_array_iter)),
           hashmap_iter: None }
  }

  fn iter_mut<'a>(&'a mut self) -> IterMut<'a, K, V> {
    IterMut { array_iter: Some(self.deref_mut().iter_mut().map(IterMut::coerce_array_iter)),
              hashmap_iter: None }
  }
}

impl<K: Eq + Hash, V> GetSize for HashMap<K, V> {
  fn get_size(&self) -> usize {
    self.len()
  }

  fn max_size(&self) -> Option<usize> {
    None
  }
}

impl<K: Eq + Hash, V> Reserve for HashMap<K, V> {
  fn reserve(n: usize) -> Self {
    Self::with_capacity(n)
  }
}

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
/// use kwap_common::Map;
///
/// let mut map = HashMap::from([("a", 1)]);
///
/// fn do_stuff(map: &impl Map<&'static str, usize>) {
///   let iter = map.iter();
/// }
/// ```
#[derive(Debug)]
pub struct Iter<'a, K: Eq + Hash, V> {
  // TODO: #[cfg(not(no_std))]?
  hashmap_iter: Option<hash_map::Iter<'a, K, V>>,
  array_iter: Option<iter::Map<slice::Iter<'a, (K, V)>, fn(&'a (K, V)) -> (&'a K, &'a V)>>,
}

impl<'a, K: Eq + Hash, V> Iter<'a, K, V> {
  #[inline(always)]
  fn coerce_array_iter((k, v): &'a (K, V)) -> (&'a K, &'a V) {
    (k, v)
  }

  fn get_iter(&mut self) -> &mut dyn Iterator<Item = (&'a K, &'a V)> {
    let (a, b) = (self.hashmap_iter.as_mut().map(|a| a as &mut _), self.array_iter.as_mut().map(|a| a as &mut _));
    a.or(b).unwrap()
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
/// use kwap_common::Map;
///
/// let mut map = HashMap::from([("a", 1)]);
///
/// fn do_stuff(map: &mut impl Map<&'static str, usize>) {
///   let iter = map.iter_mut();
/// }
/// ```
#[derive(Debug)]
pub struct IterMut<'a, K: Eq + Hash, V> {
  // TODO: #[cfg(not(no_std))]?
  hashmap_iter: Option<hash_map::IterMut<'a, K, V>>,
  array_iter: Option<iter::Map<slice::IterMut<'a, (K, V)>, fn(&'a mut (K, V)) -> (&'a K, &'a mut V)>>,
}

impl<'a, K: Eq + Hash, V> IterMut<'a, K, V> {
  #[inline(always)]
  fn coerce_array_iter((k, v): &'a mut (K, V)) -> (&'a K, &'a mut V) {
    (k, v)
  }

  fn get_iter(&mut self) -> &mut dyn Iterator<Item = (&'a K, &'a mut V)> {
    let (a, b) = (self.hashmap_iter.as_mut().map(|a| a as &mut _), self.array_iter.as_mut().map(|a| a as &mut _));
    a.or(b).unwrap()
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

  fn impls() -> (impl Map<String, String>, impl Map<String, String>, impl Map<String, String>) {
    (HashMap::<String, String>::from([("foo".into(), "bar".into())]),
     tinyvec::array_vec!([(String, String); 16] => ("foo".into(), "bar".into())),
     vec![("foo".to_string(), "bar".to_string())])
  }

  macro_rules! each_impl {
    ($work:expr) => {{
      let (hm, av, vc) = impls();
      println!("hashmap");
      $work(hm);
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
