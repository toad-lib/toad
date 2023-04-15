use java::NoUpcast;

use crate::java::{self, Nullable, Object};

/// `java.lang.Throwable`
pub struct StackTraceElement(java::lang::Object);
java::object_newtype!(StackTraceElement);
impl java::Class for StackTraceElement {
  const PATH: &'static str = "java/lang/StackTraceElement";
}

/// `java.lang.Throwable`
pub struct Throwable(java::lang::Object);

java::object_newtype!(Throwable);
impl java::Class for Throwable {
  const PATH: &'static str = "java/lang/Throwable";
}

impl Throwable {
  /// `java.lang.Throwable.getStackTrace()`
  pub fn stack_trace(&self, e: &mut java::Env) -> Vec<StackTraceElement> {
    java::Method::<Self, fn() -> Vec<StackTraceElement>>::new("getStackTrace").invoke(e, self)
  }

  /// `java.lang.Throwable.getCause()`
  pub fn cause(&self, e: &mut java::Env) -> Option<Throwable> {
    java::Method::<Self, fn() -> Nullable<Throwable>>::new("getCause").invoke(e, self)
                                                                      .into_option(e)
  }

  /// Recursively travel up the `Throwable` cause chain until one has no inner exception
  pub fn cause_iter(&self, e: &mut java::Env) -> impl Iterator<Item = Throwable> {
    struct Iter(Throwable);
    impl Iterator for Iter {
      type Item = Throwable;

      fn next(&mut self) -> Option<Self::Item> {
        let mut e = java::env();
        let e = &mut e;
        let c = self.0.cause(e);
        if let Some(c) = c.as_ref() {
          self.0 = c.downcast_ref(e).upcast_to(e);
        }
        c
      }
    }

    Iter(self.downcast_ref(e).upcast_to(e))
  }
}

impl core::fmt::Debug for Throwable {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let mut e = java::env();
    let e = &mut e;
    let traces = self.stack_trace(e);
    let traces = traces.into_iter()
                       .map(|o| o.downcast(e).to_string(e))
                       .collect::<Vec<_>>();
    write!(f, "{}\n", self.downcast_ref(e).to_string(e))?;
    self.cause_iter(e)
        .try_for_each(|cause| write!(f, "    {}\n", cause.downcast_ref(e).to_string(e)))?;
    write!(f, "\nstacktrace:\n{:#?}", traces)?;

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use crate::java::io::IOException;

  #[test]
  fn dbg() {
    let mut e = crate::test::init();
    let e = &mut e;
    let baz = IOException::new(e, "baz").to_throwable(e);
    let bar = IOException::new_caused_by(e, "bar", baz).to_throwable(e);
    let foo = IOException::new_caused_by(e, "foo", bar).to_throwable(e);

    assert_eq!(
               format!("{:?}", foo),
               format!(
      r#"
java.io.IOException: foo
    java.io.IOException: bar
    java.io.IOException: baz

stacktrace:
{:#?}"#,
      Vec::<String>::new()
    ).trim_start()
    );
  }
}
