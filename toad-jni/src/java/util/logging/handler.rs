use super::Level;
use crate::java;

/// `java.util.logging.Handler`
pub struct Handler(java::lang::Object);
java::object_newtype!(Handler);
impl java::Class for Handler {
  const PATH: &'static str = "java/util/logging/Handler";
}

/// `java.util.logging.ConsoleHandler`
pub struct ConsoleHandler(java::lang::Object);
java::object_newtype!(ConsoleHandler);
impl java::Class for ConsoleHandler {
  const PATH: &'static str = "java/util/logging/ConsoleHandler";
}

impl ConsoleHandler {
  /// `ConsoleHandler()`
  pub fn new(e: &mut java::Env) -> Self {
    static CTOR: java::Constructor<ConsoleHandler, fn()> = java::Constructor::new();
    CTOR.invoke(e)
  }

  /// `void setLevel(java.util.logging.Level)`
  pub fn set_level(&self, e: &mut java::Env, level: Level) {
    static SET_LEVEL: java::Method<ConsoleHandler, fn(Level)> = java::Method::new("setLevel");
    SET_LEVEL.invoke(e, self, level);
  }

  /// Convert to parent class `Handler`
  pub fn to_handler(self) -> Handler {
    Handler(self.0)
  }
}
