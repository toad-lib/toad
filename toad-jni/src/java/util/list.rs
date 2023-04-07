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
  /// java.util.ArrayList.get(int)
  pub fn get(&self, e: &mut java::Env, ix: i32) -> T {
    static GET: java::Method<ArrayList<java::lang::Object>, fn(i32) -> java::lang::Object> =
      java::Method::new("get");
    GET.invoke(e, self.cast_ref(), ix).upcast_to::<T>(e)
  }

  /// java.util.ArrayList.remove(int)
  pub fn remove(&self, e: &mut java::Env, ix: i32) {
    static REMOVE: java::Method<ArrayList<java::lang::Object>, fn(i32) -> java::lang::Object> =
      java::Method::new("remove");
    REMOVE.invoke(e, self.cast_ref(), ix);
  }

  /// java.util.ArrayList.clear()
  pub fn clear(&self, e: &mut java::Env) {
    static CLEAR: java::Method<ArrayList<java::lang::Object>, fn()> = java::Method::new("clear");
    CLEAR.invoke(e, self.cast_ref())
  }

  /// java.util.ArrayList.add(Object)
  pub fn append(&self, e: &mut java::Env, val: T) {
    static ADD: java::Method<ArrayList<java::lang::Object>, fn(java::lang::Object) -> bool> =
      java::Method::new("add");
    let val = val.downcast(e);
    ADD.invoke(e, self.cast_ref(), val);
  }

  /// java.util.ArrayList.add(int, Object)
  pub fn insert(&self, e: &mut java::Env, ix: i32, val: T) {
    static INSERT: java::Method<ArrayList<java::lang::Object>, fn(i32, java::lang::Object)> =
      java::Method::new("add");
    let val = val.downcast(e);
    INSERT.invoke(e, self.cast_ref(), ix, val)
  }

  /// java.util.ArrayList.size()
  pub fn size(&self, e: &mut java::Env) -> i32 {
    static SIZE: java::Method<ArrayList<java::lang::Object>, fn() -> i32> =
      java::Method::new("size");
    SIZE.invoke(e, self.cast_ref())
  }

  fn cast_ref<R>(&self) -> &ArrayList<R> {
    // SAFETY:
    // this is safe because there are no values of type `T`
    // stored in this struct; simply just casting the PhantomData
    // to a different PhantomData.
    unsafe { core::mem::transmute(self) }
  }

  fn cast<R>(self) -> ArrayList<R> {
    ArrayList(self.0, PhantomData)
  }

  /// Create a new [`ArrayList`]
  pub fn new<'local>(e: &mut java::Env<'local>) -> Self {
    static CTOR: java::Constructor<ArrayList<java::lang::Object>, fn()> = java::Constructor::new();
    CTOR.invoke(e).cast()
  }
}

impl<T> Extend<T> for ArrayList<T> where T: java::Object
{
  fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
    let mut e = java::env();
    let e = &mut e;
    iter.into_iter().for_each(|t| {
                      self.append(e, t);
                    })
  }
}

impl<T> Len for ArrayList<T> where T: java::Object
{
  const CAPACITY: Option<usize> = None;

  fn len(&self) -> usize {
    self.size(&mut java::env()) as usize
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

    let len = self.size(e) as usize;

    if desired_len == 0 {
      ArrayList::clear(self, e);
    }

    if len == 0 || desired_len >= len {
      return;
    }

    while self.len() < desired_len {
      let new_len = self.size(e) as usize;
      self.remove(e, (new_len - 1) as i32);
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
    let list = ArrayList::<T>::new(e);
    while let Some(t) = iter.next() {
      list.append(e, t);
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
      let o = self.list.get(e, self.ix as i32);
      self.ix += 1;
      Some(o)
    }
  }
}
