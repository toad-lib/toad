use core::marker::PhantomData;

use toad_array::{Filled, Reserve, Trunc};
use toad_len::Len;

use crate::java;

/// java/util/ArrayList
pub struct ArrayList<T>(java::lang::Object, PhantomData<T>);

impl<T> java::Class for ArrayList<T> where T: java::Object
{
  const PATH: &'static str = "java/util/ArrayList";
}

impl<T> java::Object for ArrayList<T> where T: java::Object
{
  fn upcast<'a, 'e>(_e: &'a mut java::Env<'e>, jobj: java::lang::Object) -> Self {
    Self(jobj, PhantomData)
  }

  fn downcast<'a, 'e>(self, _e: &'a mut java::Env<'e>) -> java::lang::Object {
    self.0
  }

  fn downcast_ref<'a, 'e>(&'a self, e: &'a mut java::Env<'e>) -> java::lang::Object {
    (&self.0).downcast_ref(e)
  }
}

impl<T> ArrayList<T> where T: java::Object
{
  /// java ArrayList constructor signature
  pub const CTOR: java::Constructor<Self, fn()> = java::Constructor::new();

  /// ArrayList.get
  pub const GET: java::Method<Self, fn(i32) -> java::lang::Object> = java::Method::new("get");

  /// Object remove(int)
  pub const REMOVE: java::Method<Self, fn(i32) -> java::lang::Object> = java::Method::new("remove");

  /// void clear()
  pub const CLEAR: java::Method<Self, fn()> = java::Method::new("clear");

  /// boolean add(Object)
  pub const ADD: java::Method<Self, fn(java::lang::Object) -> bool> = java::Method::new("add");

  /// void add(int, Object)
  pub const INSERT: java::Method<Self, fn(i32, java::lang::Object)> = java::Method::new("add");

  /// int size()
  pub const SIZE: java::Method<Self, fn() -> i32> = java::Method::new("size");

  /// Create a new [`ArrayList`]
  pub fn new<'local>(e: &mut java::Env<'local>) -> Self {
    Self::CTOR.invoke(e)
  }
}

impl<T> Extend<T> for ArrayList<T> where T: java::Object
{
  fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
    let mut e = java::env();
    let e = &mut e;
    iter.into_iter().for_each(|t| {
                      let t = t.downcast(e);
                      Self::ADD.invoke(e, self, t);
                    })
  }
}

impl<T> Len for ArrayList<T> where T: java::Object
{
  const CAPACITY: Option<usize> = None;

  fn len(&self) -> usize {
    Self::SIZE.invoke(&mut java::env(), self) as usize
  }

  fn is_full(&self) -> bool {
    false
  }
}

impl<T> Trunc for ArrayList<T> where T: java::Object
{
  fn trunc(&mut self, desired_len: usize) -> () {
    let mut e = java::env();
    let e = &mut e;

    let len = Self::SIZE.invoke(e, self) as usize;

    if desired_len == 0 {
      Self::CLEAR.invoke(e, self);
    }

    if len == 0 || desired_len >= len {
      return;
    }

    while self.len() < desired_len {
      let new_len = Self::SIZE.invoke(e, self) as usize;
      Self::REMOVE.invoke(e, self, (new_len - 1) as i32);
    }
  }
}

impl<T> IntoIterator for ArrayList<T> where T: java::Object
{
  type Item = T;
  type IntoIter = ArrayListIter<T>;

  fn into_iter(self) -> Self::IntoIter {
    ArrayListIter { ix: 0,
                    len: self.len(),
                    list: self }
  }
}

impl<T> FromIterator<T> for ArrayList<T> where T: java::Object
{
  fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
    let mut e = java::env();
    let e = &mut e;

    let mut iter = iter.into_iter();
    let list = ArrayList::new(e);
    while let Some(t) = iter.next() {
      let t = t.downcast(e);
      Self::ADD.invoke(e, &list, t);
    }

    list
  }
}

impl<T> Default for ArrayList<T> where T: java::Object
{
  fn default() -> Self {
    Self::new(&mut java::env())
  }
}

impl<T> Filled<T> for ArrayList<T> where T: java::Object
{
  fn filled_using<F>(_: F) -> Option<Self>
    where F: Fn() -> T
  {
    None
  }
}

impl<T> Reserve for ArrayList<T> where T: java::Object {}

/// [`ArrayList`] owned iterator
pub struct ArrayListIter<T> {
  list: ArrayList<T>,
  ix: usize,
  len: usize,
}

impl<T> Iterator for ArrayListIter<T> where T: java::Object
{
  type Item = T;

  fn next(&mut self) -> Option<Self::Item> {
    let mut e = java::env();
    let e = &mut e;

    if self.ix == self.len {
      None
    } else {
      let o = ArrayList::GET.invoke(e, &self.list, self.ix as i32);
      self.ix += 1;
      Some(o.upcast_to::<T>(e))
    }
  }
}
