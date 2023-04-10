use crate::java::{self, Nullable};

/// `java.lang.System`
#[derive(Debug, Clone, Copy)]
pub struct System;

impl System {
  /// `String java.lang.System.getenv(String)`
  pub fn get_env(e: &mut java::Env, key: impl ToString) -> Option<String> {
    static GETENV: java::StaticMethod<System, fn(String) -> Nullable<String>> =
      java::StaticMethod::new("getenv");
    GETENV.invoke(e, key.to_string()).into_option(e)
  }

  /// `String java.lang.System.getProperty(String)`
  pub fn get_property(e: &mut java::Env, key: impl ToString) -> Option<String> {
    static GET_PROPERTY: java::StaticMethod<System, fn(String) -> Nullable<String>> =
      java::StaticMethod::new("getProperty");
    GET_PROPERTY.invoke(e, key.to_string()).into_option(e)
  }

  /// `String java.lang.System.setProperty(String)`
  ///
  /// Returns `Some(val)` when there was a previous value.
  pub fn set_property(e: &mut java::Env,
                      key: impl ToString,
                      value: impl ToString)
                      -> Option<String> {
    static SET_PROPERTY: java::StaticMethod<System, fn(String, String) -> Nullable<String>> =
      java::StaticMethod::new("setProperty");
    SET_PROPERTY.invoke(e, key.to_string(), value.to_string())
                .into_option(e)
  }

  /// `java.lang.System.gc()`
  pub fn gc(e: &mut java::Env) {
    static GC: java::StaticMethod<System, fn()> = java::StaticMethod::new("gc");
    GC.invoke(e)
  }

  /// `java.lang.System.load(String)`
  pub fn load_library_file(e: &mut java::Env, filename: impl ToString) {
    static LOAD: java::StaticMethod<System, fn(String)> = java::StaticMethod::new("load");
    LOAD.invoke(e, filename.to_string())
  }

  /// `java.lang.System.loadLibrary(String)`
  pub fn load_library(e: &mut java::Env, libname: impl ToString) {
    static LOAD_LIBRARY: java::StaticMethod<System, fn(String)> =
      java::StaticMethod::new("loadLibrary");
    LOAD_LIBRARY.invoke(e, libname.to_string())
  }

  /// `java.lang.System.console()`
  pub fn console(e: &mut java::Env) -> Nullable<java::io::Console> {
    static CONSOLE: java::StaticMethod<System, fn() -> Nullable<java::io::Console>> =
      java::StaticMethod::new("console");
    CONSOLE.invoke(e)
  }

  /// `java.lang.System.exit(int)`
  pub fn exit(e: &mut java::Env, status: i32) {
    static EXIT: java::StaticMethod<System, fn(i32)> = java::StaticMethod::new("exit");
    EXIT.invoke(e, status)
  }
}

impl java::Class for System {
  const PATH: &'static str = "java/lang/System";
}

impl java::Object for System {
  fn upcast(_: &mut java::Env, _: java::lang::Object) -> Self {
    panic!("java.lang.System cannot be instantiated.")
  }

  fn downcast(self, _: &mut java::Env) -> java::lang::Object {
    panic!("java.lang.System cannot be instantiated.")
  }

  fn downcast_ref(&self, _: &mut java::Env) -> java::lang::Object {
    panic!("java.lang.System cannot be instantiated.")
  }
}
