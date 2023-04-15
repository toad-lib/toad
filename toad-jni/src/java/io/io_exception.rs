use crate::java::{self, lang::Throwable, Object};

/// `java.io.IOException`
pub struct IOException(java::lang::Object);

impl IOException {
  /// [`IOException(String)`](https://docs.oracle.com/en/java/javase/19/docs/api/java.base/java/io/IOException.html#<init>(java.lang.String))
  pub fn new(e: &mut java::Env, message: impl ToString) -> Self {
    static CTOR: java::Constructor<IOException, fn(String)> = java::Constructor::new();
    CTOR.invoke(e, message.to_string())
  }

  /// [`IOException(String, Throwable)`](https://docs.oracle.com/en/java/javase/19/docs/api/java.base/java/io/IOException.html#<init>(java.lang.String,java.lang.Throwable))
  pub fn new_caused_by(e: &mut java::Env, message: impl ToString, err: Throwable) -> Self {
    static CTOR: java::Constructor<IOException, fn(String, Throwable)> = java::Constructor::new();
    CTOR.invoke(e, message.to_string(), err)
  }

  /// Cast self to Throwable
  pub fn to_throwable(&self, e: &mut java::Env) -> Throwable {
    self.downcast_ref(e).upcast_to::<Throwable>(e)
  }
}

impl core::fmt::Debug for IOException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.to_throwable(&mut java::env()))
    }
}

java::object_newtype!(IOException);
impl java::Class for IOException {
  const PATH: &'static str = "java/io/IOException";
}

impl<StepError> toad::platform::PlatformError<StepError, Throwable> for IOException where StepError: core::fmt::Debug {
    fn msg_to_bytes(e: toad_msg::to_bytes::MessageToBytesError) -> Self {
        Self::new(&mut java::env(), format!("{:?}", e))
    }

    fn step(e: StepError) -> Self {
        Self::new(&mut java::env(), format!("{:?}", e))
    }

    fn socket(e: Throwable) -> Self {
        Self::new_caused_by(&mut java::env(), "", e)
    }

    fn clock(e: embedded_time::clock::Error) -> Self {
        Self::new(&mut java::env(), format!("{:?}", e))
    }
}
