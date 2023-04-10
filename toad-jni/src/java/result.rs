use jni::errors::Result;

use crate::java;

/// Result extensions
pub trait ResultExt<T> {
  /// If a java exception occurred, toString it and panic
  fn unwrap_java(self, e: &mut java::Env) -> T;
}

impl<T> ResultExt<T> for Result<T> {
  fn unwrap_java(self, e: &mut java::Env) -> T {
    use jni::errors::Error::*;

    match self {
      | Ok(t) => t,
      | Err(JavaException) => {
        let ex = e.exception_occurred().unwrap();
        let exo = java::lang::Object::from_local(e, ex);
        e.exception_clear().unwrap();
        panic!("{}", exo.to_string(e));
      },
      | o => o.unwrap(),
    }
  }
}
