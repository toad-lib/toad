use crate::java;

/// `java.io.PrintStream`
pub struct PrintStream(java::lang::Object);

java::object_newtype!(PrintStream);
impl java::Class for PrintStream {
  const PATH: &'static str = "java/io/PrintStream";
}

impl PrintStream {
  /// `java.io.PrintStream.printf(String, java.lang.Object...)`
  pub fn printf(&self, e: &mut java::Env, format: impl ToString, args: Vec<java::lang::Object>) {
    static PRINTF: java::Method<PrintStream, fn(String, Vec<java::lang::Object>) -> PrintStream> =
      java::Method::new("printf");
    PRINTF.invoke(e, self, format.to_string(), args);
  }
}
