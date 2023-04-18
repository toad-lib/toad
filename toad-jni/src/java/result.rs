use java::lang::Throwable;
use jni::objects::{JObject, JThrowable};
use jni::sys::jobject;

use crate::java::{self, Object};

/// Handle a `Result<impl java::Object, Throwable>` in a native
/// method implementation.
///
/// if `Err` throw the [`Throwable`] contained in the result.
///
/// if `Ok` invoke [`java::Object::yield_to_java`].
pub trait ResultYieldToJavaOrThrow {
  #[allow(missing_docs)]
  fn yield_to_java_or_throw(self, e: &mut java::Env) -> jobject;
}

impl<T> ResultYieldToJavaOrThrow for Result<T, Throwable> where T: java::Object
{
  fn yield_to_java_or_throw(self, e: &mut java::Env) -> jobject {
    self.map(|ok| ok.yield_to_java(e))
        .map_err(|err| {
          let err = JThrowable::from(err.downcast(e).to_local(e));
          e.throw(err).unwrap()
        })
        .unwrap_or(*JObject::null())
  }
}

/// [`toad_jni::errors::Result`] interop helpers
pub trait ResultExt<T> {
  /// If a java exception occurred, toString it and panic
  fn unwrap_java(self, e: &mut java::Env) -> T;

  /// If a java exception occurred, convert to [`java::lang::Throwable`]
  fn to_throwable(self, e: &mut java::Env) -> Result<T, Throwable>;
}

impl<T> ResultExt<T> for jni::errors::Result<T> {
  fn unwrap_java(self, e: &mut java::Env) -> T {
    use jni::errors::Error::*;

    match self {
      | Ok(t) => t,
      | Err(JavaException) => {
        let ex = e.exception_occurred().unwrap();
        let exo = java::lang::Object::from_local(e, ex);
        e.exception_clear().unwrap();
        panic!("{:?}", exo.upcast_to::<java::lang::Throwable>(e));
      },
      | o => o.unwrap(),
    }
  }

  fn to_throwable(self, e: &mut java::Env) -> Result<T, java::lang::Throwable> {
    use jni::errors::Error::*;

    match self {
      | Err(JavaException) => {
        let ex = e.exception_occurred().unwrap();
        e.exception_clear().unwrap();
        let exo = java::lang::Object::from_local(e, ex);
        Err(java::lang::Throwable::upcast(e, exo))
      },
      | o => Ok(o.unwrap()),
    }
  }
}
