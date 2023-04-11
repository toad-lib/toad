use jni::errors::Result;

use crate::java::{self, Object, Signature};

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
        let exo = exo.upcast_to::<java::lang::Throwable>(e);
        let traces = exo.get_stack_trace(e);
        panic!("{}\ntrace:\n{:#?}",
               exo.downcast(e).to_string(e),
               traces.into_iter()
                     .map(|o| o.downcast(e).to_string(e))
                     .collect::<Vec<_>>());
      },
      | o => o.unwrap(),
    }
  }
}
