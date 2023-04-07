use core::marker::PhantomData;

use crate::java;

/// java/util/Optional
pub struct Optional<T>(java::lang::Object, PhantomData<T>);

impl<T> Optional<T> where T: java::Object
{
  fn cast<R>(self) -> Optional<R> {
    Optional(self.0, PhantomData)
  }

  fn cast_ref<R>(&self) -> &Optional<R> {
    // SAFETY:
    // this is safe because there are no values of type `T`
    // stored in this struct; simply just casting the PhantomData
    // to a different PhantomData.
    unsafe { core::mem::transmute(self) }
  }

  /// java.util.Optional$of
  pub fn of(e: &mut java::Env, t: T) -> Self {
    static OF: java::StaticMethod<Optional<java::lang::Object>,
                                    fn(java::lang::Object) -> Optional<java::lang::Object>> =
      java::StaticMethod::new("of");
    let t = t.downcast(e);
    OF.invoke(e, t).cast()
  }

  /// Create an empty instance of `Optional<T>`
  pub fn empty<'a>(e: &mut java::Env<'a>) -> Self {
    static EMPTY: java::StaticMethod<Optional<java::lang::Object>,
                                       fn() -> Optional<java::lang::Object>> =
      java::StaticMethod::new("empty");
    EMPTY.invoke(e).cast()
  }

  /// Is this Optional empty? (equivalent to [`Option.is_none`])
  pub fn is_empty<'a>(&self, e: &mut java::Env<'a>) -> bool {
    static IS_EMPTY: java::Method<Optional<java::lang::Object>, fn() -> bool> =
      java::Method::new("isEmpty");
    IS_EMPTY.invoke(e, self.cast_ref())
  }

  /// Extract the value from the optional, throwing a Java exception if it was empty.
  ///
  /// (equivalent to [`Option.unwrap`])
  pub fn get<'a>(&self, e: &mut java::Env<'a>) -> T {
    static GET: java::Method<Optional<java::lang::Object>, fn() -> java::lang::Object> =
      java::Method::new("get");
    GET.invoke(e, self.cast_ref()).upcast_to::<T>(e)
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
