use jni::objects::{GlobalRef, JObject};
use jni::sys::jlong;
use jni::JNIEnv;

use crate::convert::Object;
use crate::Sig;

/// java/time/Duration
pub struct Duration(GlobalRef);

impl Duration {
  /// Fully qualified path
  pub const PATH: &'static str = "java/time/Duration";

  /// java.time.Duration$ofMillis
  pub const OF_MILLIS: Sig = Sig::new().arg(Sig::LONG).returning(Sig::class(Self::PATH));

  /// java.time.Duration$toMillis
  pub const TO_MILLIS: Sig = Sig::new().returning(Sig::LONG);

  /// java.time.Duration$ofMillis
  pub fn of_millis<'a>(e: &mut JNIEnv<'a>, millis: jlong) -> Self {
    let o = e.call_static_method(Self::PATH, "ofMillis", Self::OF_MILLIS, &[millis.into()])
             .unwrap()
             .l()
             .unwrap();
    Self(e.new_global_ref(o).unwrap())
  }

  /// java.time.Duration$ofMillis
  pub fn to_millis<'a>(&self, e: &mut JNIEnv<'a>) -> jlong {
    e.call_method(&self.0, "toMillis", Self::TO_MILLIS, &[])
     .unwrap()
     .j()
     .unwrap()
  }
}

impl Object for Duration {
  fn from_java(jobj: GlobalRef) -> Self {
    Self(jobj)
  }

  fn to_java(self) -> GlobalRef {
    self.0
  }
}
