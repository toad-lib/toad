use std::marker::PhantomData;

use crate::java;

/// Helper type that has `T`'s [`java::Class::PATH`] but
/// does not invoke [`java::Object::upcast`] on it, preserving
/// the [`java::lang::Object`].
pub struct NoUpcast<T>(java::lang::Object, PhantomData<T>);

impl<T> NoUpcast<T> where T: java::Class
{
  /// Unwrap, yielding the object reference
  pub fn object(self) -> java::lang::Object {
    self.0
  }

  /// Wraps an object with `NoUpcast<T>`
  pub fn from_object(o: java::lang::Object) -> Self {
    Self(o, PhantomData)
  }
}

impl<T> java::Class for NoUpcast<T> where T: java::Class
{
  const PATH: &'static str = T::PATH;
}

impl<T> java::Object for NoUpcast<T> where T: java::Class
{
  fn upcast(_: &mut java::Env, jobj: java::lang::Object) -> Self {
    Self(jobj, PhantomData)
  }

  fn downcast(self, _: &mut java::Env) -> java::lang::Object {
    self.object()
  }

  fn downcast_ref(&self, e: &mut java::Env) -> java::lang::Object {
    self.0.downcast_ref(e)
  }
}
