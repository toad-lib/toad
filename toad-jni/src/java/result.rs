use jni::objects::JObject;

use crate::java::{self, Object, Signature};

/// Result extensions
pub trait ResultExt<T> {
  /// If a java exception occurred, toString it and panic
  fn unwrap_java(self, e: &mut java::Env) -> T;

  /// If a java exception occurred, convert to [`java::lang::Throwable`]
  fn to_throwable(self, e: &mut java::Env) -> Result<T, java::lang::Throwable>;
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
