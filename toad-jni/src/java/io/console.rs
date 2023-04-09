use crate::java::{self, NoUpcast};

/// `java.io.Console`
pub struct Console(java::lang::Object);

impl Console {
  /// `java.io.Console.printf(String, java.lang.Object...)`
  pub fn printf(&self,
                e: &mut java::Env,
                fmt: impl ToString,
                args: Vec<java::lang::Object>)
                -> &Self {
    static PRINTF: java::Method<Console, fn(String, Vec<java::lang::Object>) -> NoUpcast<Console>> =
      java::Method::new("printf");
    PRINTF.invoke(e, &self, fmt.to_string(), args);
    self
  }

  /// `java.io.Console.readLine(String, java.lang.Object...)`
  pub fn readline(&self,
                  e: &mut java::Env,
                  fmt: impl ToString,
                  args: Vec<java::lang::Object>)
                  -> String {
    static READLINE: java::Method<Console, fn(String, Vec<java::lang::Object>) -> String> =
      java::Method::new("readLine");
    READLINE.invoke(e, &self, fmt.to_string(), args)
  }
}

java::object_newtype!(Console);
impl java::Class for Console {
  const PATH: &'static str = "java/io/Console";
}
