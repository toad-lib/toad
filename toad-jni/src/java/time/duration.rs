use crate::java;

/// java/time/Duration
pub struct Duration(java::lang::Object);

impl Duration {
  /// java.time.Duration.ofMillis(long)
  pub fn of_millis(e: &mut java::Env, millis: i64) -> Self {
    static OF_MILLIS: java::StaticMethod<Duration, fn(i64) -> Duration> =
      java::StaticMethod::new("ofMillis");
    OF_MILLIS.invoke(e, millis)
  }

  /// java.time.Duration.toMillis()
  pub fn to_millis(&self, e: &mut java::Env) -> i64 {
    static TO_MILLIS: java::Method<Duration, fn() -> i64> = java::Method::new("toMillis");
    TO_MILLIS.invoke(e, self)
  }
}

impl java::Class for Duration {
  const PATH: &'static str = "java/time/Duration";
}

impl java::Object for Duration {
  fn upcast(_: &mut java::Env, jobj: java::lang::Object) -> Self {
    Self(jobj)
  }

  fn downcast(self, _: &mut java::Env) -> java::lang::Object {
    self.0
  }

  fn downcast_ref(&self, e: &mut java::Env) -> java::lang::Object {
    self.0.downcast_ref(e)
  }
}
