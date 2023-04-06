use core::marker::PhantomData;

use crate::java;

/// java/util/Optional
pub struct Optional<T>(java::lang::Object, PhantomData<T>);

impl<T> Optional<T> where T: java::Object
{
  /// Fully qualified class path

  /// java.util.Optional$of
  pub const OF: java::StaticMethod<Self, fn(java::lang::Object) -> Self> =
    java::StaticMethod::new("of");

  /// java.util.Optional$empty
  pub const EMPTY: java::StaticMethod<Self, fn() -> Self> = java::StaticMethod::new("empty");

  /// java.util.Optional$get
  pub const GET: java::Method<Self, fn() -> java::lang::Object> = java::Method::new("get");

  /// java.util.Optional$isEmpty
  pub const IS_EMPTY: java::Method<Self, fn() -> bool> = java::Method::new("isEmpty");

  /// Given a value of type `T`, wrap it in `Optional`.
  pub fn of<'a>(e: &mut java::Env<'a>, t: T) -> Self {
    let o = t.downcast(e);
    Self::OF.invoke(e, o)
  }

  /// Create an empty instance of `Optional<T>`
  pub fn empty<'a>(e: &mut java::Env<'a>) -> Self {
    Self::EMPTY.invoke(e)
  }

  /// Is this Optional empty? (equivalent to [`Option.is_none`])
  pub fn is_empty<'a>(&self, e: &mut java::Env<'a>) -> bool {
    Self::IS_EMPTY.invoke(e, self)
  }

  /// Extract the value from the optional, throwing a Java exception if it was empty.
  ///
  /// (equivalent to [`Option.unwrap`])
  pub fn get<'a>(&self, e: &mut java::Env<'a>) -> T {
    let got = Self::GET.invoke(e, self);
    got.upcast_to::<T>(e)
  }

  /// Infallibly convert this java `Optional<T>` to a rust `Option<T>`.
  pub fn to_option<'a>(self, e: &mut java::Env<'a>) -> Option<T> {
    if self.is_empty(e) {
      None
    } else {
      Some(self.get(e))
    }
  }

  /// Infallibly convert create a java `Optional<T>` from a rust `Option<T>`.
  pub fn from_option<'a>(o: Option<T>, e: &mut java::Env<'a>) -> Self {
    o.map(|t| Self::of(e, t)).unwrap_or_else(|| Self::empty(e))
  }
}

impl<T> java::Class for Optional<T> where T: java::Object
{
  const PATH: &'static str = "java/util/Optional";
}

impl<T> java::Object for Optional<T> where T: java::Object
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
