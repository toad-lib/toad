use super::Object;

/// An object with a known class definition
pub trait Class: Object {
  /// The fully qualified java class path (slash-separated)
  ///
  /// ```
  /// use toad_jni::java;
  ///
  /// // com.mypkg.Foo
  /// struct Foo(java::lang::Object);
  ///
  /// java::object_newtype!(Foo);
  ///
  /// impl java::Class for Foo {
  ///   const PATH: &'static str = "com/mypkg/Foo";
  /// }
  /// ```
  const PATH: &'static str;
}
