use core::marker::PhantomData;

use jni::objects::{GlobalRef, JObject, JValueGen};
use jni::JNIEnv;

use crate::{convert, Sig};

/// java/util/Optional
pub struct Optional<T>(GlobalRef, PhantomData<T>);

impl<T> Optional<T> where T: convert::Object
{
  /// Fully qualified class path
  pub const PATH: &'static str = "java/util/Optional";

  /// java.util.Optional$of
  pub const OF: Sig = Sig::new().arg(Sig::class("java/lang/Object"))
                                .returning(Sig::class(Self::PATH));

  /// java.util.Optional$empty
  pub const EMPTY: Sig = Sig::new().returning(Sig::class(Self::PATH));

  /// java.util.Optional$get
  pub const GET: Sig = Sig::new().returning(Sig::class("java/lang/Object"));

  /// java.util.Optional$isEmpty
  pub const IS_EMPTY: Sig = Sig::new().returning(Sig::BOOL);

  /// Given a value of type `T`, wrap it in `Optional`.
  pub fn of<'a>(e: &mut JNIEnv<'a>, t: T) -> Self {
    let o = e.call_static_method(Self::PATH,
                                 "of",
                                 Self::OF,
                                 &[JValueGen::Object(t.to_java().as_obj())])
             .unwrap()
             .l()
             .unwrap();
    let g = e.new_global_ref(&o).unwrap();
    Self(g, PhantomData)
  }

  /// Create an empty instance of `Optional<T>`
  pub fn empty<'a>(e: &mut JNIEnv<'a>) -> Self {
    let o = e.call_static_method(Self::PATH, "empty", Self::EMPTY, &[])
             .unwrap()
             .l()
             .unwrap();
    let g = e.new_global_ref(&o).unwrap();
    Self(g, PhantomData)
  }

  /// Is this Optional empty? (equivalent to [`Option.is_none`])
  pub fn is_empty<'a>(&self, e: &mut JNIEnv<'a>) -> bool {
    e.call_method(&self.0, "isEmpty", Self::IS_EMPTY, &[])
     .unwrap()
     .z()
     .unwrap()
  }

  /// Extract the value from the optional, throwing a Java exception if it was empty.
  ///
  /// (equivalent to [`Option.unwrap`])
  pub fn get<'a>(&self, e: &mut JNIEnv<'a>) -> T {
    let o = e.call_method(&self.0, "get", Self::GET, &[])
             .unwrap()
             .l()
             .unwrap();
    let g = e.new_global_ref(&o).unwrap();
    T::from_java(g)
  }

  /// Infallibly convert this java `Optional<T>` to a rust `Option<T>`.
  pub fn to_option<'a>(self, e: &mut JNIEnv<'a>) -> Option<T> {
    if self.is_empty(e) {
      None
    } else {
      Some(self.get(e))
    }
  }

  /// Infallibly convert create a java `Optional<T>` from a rust `Option<T>`.
  pub fn from_option<'a>(o: Option<T>, e: &mut JNIEnv<'a>) -> Self {
    o.map(|t| Self::of(e, t)).unwrap_or_else(|| Self::empty(e))
  }
}

impl<T> convert::Object for Optional<T> where T: convert::Object
{
  fn from_java(jobj: jni::objects::GlobalRef) -> Self {
    Self(jobj, PhantomData)
  }

  fn to_java(self) -> jni::objects::GlobalRef {
    self.0
  }
}
