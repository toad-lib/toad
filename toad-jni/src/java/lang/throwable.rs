use java::NoUpcast;

use crate::java;

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
    java::Method::<Self, fn() -> Vec<StackTraceElement>>::new_overrideable("getStackTrace").invoke(e, self)
  }
}
