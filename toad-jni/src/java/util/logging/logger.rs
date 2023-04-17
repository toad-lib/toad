use super::Level;
use crate::java;

/// `java.util.logging.Logger`
pub struct Logger(java::lang::Object);

impl Logger {
  /// `java.util.logging.Logger`
  pub fn get_logger(e: &mut java::Env, name: impl ToString) -> Logger {
    static GET_LOGGER: java::StaticMethod<Logger, fn(String) -> Logger> =
      java::StaticMethod::new("getLogger");
    GET_LOGGER.invoke(e, name.to_string())
  }

  /// `java.util.logging.Logger.log`
  pub fn log(&self, e: &mut java::Env, level: Level, msg: impl ToString) {
    static LOG: java::Method<Logger, fn(Level, String)> = java::Method::new("log");
    LOG.invoke(e, self, level, msg.to_string())
  }
}

java::object_newtype!(Logger);
impl java::Class for Logger {
  const PATH: &'static str = "java/util/logging/Logger";
}
