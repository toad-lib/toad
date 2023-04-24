use crate::java;

/// `java.util.logging.Formatter`
pub struct Formatter(java::lang::Object);

java::object_newtype!(Formatter);
impl java::Class for Formatter {
  const PATH: &'static str = "java/util/logging/Formatter";
}
