use super::{Handler, Level};
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

  /// `void setLevel(java.util.logging.Level)`
  pub fn set_level(&self, e: &mut java::Env, level: Level) {
    static SET_LEVEL: java::Method<Logger, fn(Level)> = java::Method::new("setLevel");
    SET_LEVEL.invoke(e, self, level);
  }

  /// `void setUseParentHandlers(boolean)`
  pub fn use_parent_handlers(&self, e: &mut java::Env, should_do_it_question_mark: bool) {
    static SET_USE_PARENT_HANDLERS: java::Method<Logger, fn(bool)> =
      java::Method::new("setUseParentHandlers");
    SET_USE_PARENT_HANDLERS.invoke(e, self, should_do_it_question_mark);
  }

  /// `void getUseParentHandlers()`
  pub fn uses_parent_handlers(&self, e: &mut java::Env) -> bool {
    static GET_USE_PARENT_HANDLERS: java::Method<Logger, fn() -> bool> =
      java::Method::new("getUseParentHandlers");
    GET_USE_PARENT_HANDLERS.invoke(e, self)
  }

  /// `void addHandler(Handler h)`
  pub fn add_handler(&self, e: &mut java::Env, h: Handler) {
    static ADD_HANDLER: java::Method<Logger, fn(Handler)> = java::Method::new("addHandler");
    ADD_HANDLER.invoke(e, self, h);
  }
}

java::object_newtype!(Logger);
impl java::Class for Logger {
  const PATH: &'static str = "java/util/logging/Logger";
}
