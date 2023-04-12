use java::NoUpcast;

use crate::java::{self, Object};

/// `java.lang.Throwable`
pub struct StackTraceElement(java::lang::Object);
java::object_newtype!(StackTraceElement);
impl java::Class for StackTraceElement {
  const PATH: &'static str = "java/lang/StackTraceElement";
}

/// `java.lang.Throwable`
pub struct Throwable(java::lang::Object);

java::object_newtype!(Throwable);
impl java::Class for Throwable {
  const PATH: &'static str = "java/lang/Throwable";
}

impl Throwable {
  /// `java.lang.Throwable.getStackTrace()`
  pub fn get_stack_trace(&self, e: &mut java::Env) -> Vec<StackTraceElement> {
    java::Method::<Self, fn() -> Vec<StackTraceElement>>::new("getStackTrace").invoke(e, self)
  }
}

impl core::fmt::Debug for Throwable {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let mut e = java::env();
    let e = &mut e;
    let traces = self.get_stack_trace(e);
    write!(f,
           "{}\ntrace:\n{:#?}",
           self.downcast_ref(e).to_string(e),
           traces.into_iter()
                 .map(|o| o.downcast(e).to_string(e))
                 .collect::<Vec<_>>())
  }
}
