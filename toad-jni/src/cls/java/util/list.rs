use core::marker::PhantomData;

use jni::objects::GlobalRef;
use toad_array::{Filled, Reserve, Trunc};
use toad_len::Len;

#[allow(unused_imports)]
use crate::convert::Object;
use crate::{convert, global, Sig};

/// java/util/ArrayList
pub struct ArrayList<T>(GlobalRef, PhantomData<T>);

impl<T> ArrayList<T> where T: convert::Object
{
  /// java ArrayList class path
  pub const ID: &'static str = "java/util/ArrayList";

  /// java ArrayList constructor signature
  pub const CTOR: Sig = Sig::new().returning(Sig::VOID);

  /// Create a new [`ArrayList`]
  pub fn new() -> Self {
    let mut env = global::env();
    let cls = env.find_class(Self::ID).unwrap();
    let obj = env.new_object(cls, Self::CTOR, &[]).unwrap();
    Self(env.new_global_ref(obj).unwrap(), PhantomData)
  }

  /// Obtain an iterator over the [`Object`]s in this [`ArrayList`] from a reference
  pub fn iter<'a>(&'a self) -> ArrayListIterRef<'a, T> {
    ArrayListIterRef { list: &self,
                       ix: 0,
                       len: self.len() }
  }

  /// Obtain a reference to the inner [`jni::objects::JList`] ptr
  pub fn list<'obj, 'list>(&'obj self) -> jni::objects::JList<'obj, 'list, 'list>
    where 'obj: 'list
  {
    jni::objects::JList::from_env(&mut global::env(), &self.0).unwrap()
  }
}

impl<T> Extend<T> for ArrayList<T> where T: convert::Object
{
  fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
    iter.into_iter().for_each(|t| {
                      self.list()
                          .add(&mut global::env(), t.to_java().as_obj())
                          .unwrap();
                    })
  }
}

impl<T> Len for ArrayList<T> where T: convert::Object
{
  const CAPACITY: Option<usize> = None;

  fn len(&self) -> usize {
    self.list().size(&mut global::env()).unwrap() as usize
  }

  fn is_full(&self) -> bool {
    false
  }
}

impl<T> Trunc for ArrayList<T> where T: convert::Object
{
  fn trunc(&mut self, len: usize) -> () {
    while self.len() < len {
      self.list().pop(&mut global::env()).unwrap();
    }
  }
}

impl<T> IntoIterator for ArrayList<T> where T: convert::Object
{
  type Item = T;
  type IntoIter = ArrayListIter<T>;

  fn into_iter(self) -> Self::IntoIter {
    ArrayListIter { ix: 0,
                    len: self.len(),
                    list: self }
  }
}

impl<T> FromIterator<T> for ArrayList<T> where T: convert::Object
{
  fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
    let mut iter = iter.into_iter();
    let list = ArrayList::new();
    while let Some(t) = iter.next() {
      list.list()
          .add(&mut global::env(), t.to_java().as_obj())
          .unwrap();
    }

    list
  }
}

impl<T> Default for ArrayList<T> where T: convert::Object
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T> Filled<T> for ArrayList<T> where T: convert::Object
{
  fn filled_using<F>(_: F) -> Option<Self>
    where F: Fn() -> T
  {
    None
  }
}

impl<T> Reserve for ArrayList<T> where T: convert::Object {}

/// [`ArrayList`] owned iterator
pub struct ArrayListIter<T> {
  list: ArrayList<T>,
  ix: usize,
  len: usize,
}

impl<T> Iterator for ArrayListIter<T> where T: convert::Object
{
  type Item = T;

  fn next(&mut self) -> Option<Self::Item> {
    if self.ix == self.len {
      None
    } else {
      let t = self.list
                  .list()
                  .get(&mut global::env(), self.ix as i32)
                  .unwrap();
      self.ix += 1;
      t.map(convert::Object::from_jobject)
    }
  }
}

/// [`ArrayList`] reference iterator
pub struct ArrayListIterRef<'a, T> {
  list: &'a ArrayList<T>,
  ix: usize,
  len: usize,
}

impl<'a, T> Iterator for ArrayListIterRef<'a, T> where T: convert::Object
{
  type Item = T;

  fn next(&mut self) -> Option<Self::Item> {
    if self.ix == self.len {
      None
    } else {
      let t = self.list
                  .list()
                  .get(&mut global::env(), self.ix as i32)
                  .unwrap();
      self.ix += 1;
      t.map(|obj| T::from_jobject(obj))
    }
  }
}
