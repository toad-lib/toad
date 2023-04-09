use std::marker::PhantomData;

use crate::java;

/// Helper type that wraps a type `T` without
/// invoking [`java::Object::upcast`] on it, preventing
/// potential NullPointerExceptions.
pub struct Nullable<T>(java::lang::Object, PhantomData<T>);

impl<T> Nullable<T> where T: java::Class
{
  /// Convert this to [`Option`]`<T>`, invoking [`java::Object::upcast`]
  /// _only_ if the object reference is non-null.
  pub fn into_option(self, e: &mut java::Env) -> Option<T> {
    if self.0.is_null() {
      None
    } else {
      Some(self.0.upcast_to::<T>(e))
    }
  }
}

impl<T> java::Class for Nullable<T> where T: java::Class
{
  const PATH: &'static str = T::PATH;
}

impl<T> java::Object for Nullable<T> where T: java::Class
{
  fn upcast(_: &mut java::Env, jobj: java::lang::Object) -> Self {
    Self(jobj, PhantomData)
  }

  fn downcast(self, _: &mut java::Env) -> java::lang::Object {
    self.0
  }

  fn downcast_ref(&self, e: &mut java::Env) -> java::lang::Object {
    self.0.downcast_ref(e)
  }
}
