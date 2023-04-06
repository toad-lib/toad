use super::Object;

/// An object with a known class definition
pub trait Class: Object {
  /// The fully qualified java class path
  const PATH: &'static str;
}
