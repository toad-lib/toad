mod primitive_wrappers;

#[doc(inline)]
pub use primitive_wrappers::{Bool, Byte, Char, Double, Float, Integer, Long, Short};

mod object;

#[doc(inline)]
pub use object::Object;

use crate::java;

impl java::Class for String {
  const PATH: &'static str = "java/lang/String";
}

impl java::Object for String {
  fn upcast(e: &mut java::Env, jobj: Object) -> Self {
    let jstring = <&jni::objects::JString>::from(jobj.as_local());
    let javastr = e.get_string(jstring).unwrap();
    javastr.into()
  }

  fn downcast(self, e: &mut java::Env) -> Object {
    self.downcast_ref(e)
  }

  fn downcast_ref(&self, e: &mut java::Env) -> Object {
    let str_ = e.new_string(self).unwrap();
    Object::from_local(e, str_)
  }
}
