use crate::java;

/// java/time/Duration
pub struct Duration(java::lang::Object);

impl Duration {
  /// java.time.Duration.ofMillis(long)
  pub fn of_millis<'a>(e: &mut java::Env<'a>, millis: i64) -> Self {
    static OF_MILLIS: java::StaticMethod<Duration, fn(i64) -> Duration> =
      java::StaticMethod::new("ofMillis");
    OF_MILLIS.invoke(e, millis)
  }

  /// java.time.Duration.toMillis()
  pub fn to_millis<'a>(&self, e: &mut java::Env<'a>) -> i64 {
    static TO_MILLIS: java::Method<Duration, fn() -> i64> = java::Method::new("toMillis");
    TO_MILLIS.invoke(e, self)
  }
}

impl java::Class for Duration {
  const PATH: &'static str = "java/time/Duration";
}

impl java::Object for Duration {
  fn upcast<'a, 'e>(_: &'a mut java::Env<'e>, jobj: java::lang::Object) -> Self {
    Self(jobj)
  }

  fn downcast<'a, 'e>(self, _: &'a mut java::Env<'e>) -> java::lang::Object {
    self.0
  }

  fn downcast_ref<'a, 'e>(&'a self, e: &'a mut java::Env<'e>) -> java::lang::Object {
    (&self.0).downcast_ref(e)
  }
}
