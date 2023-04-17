use crate::java::{self, NoUpcast};

/// `java.util.logging.Level`
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum Level {
  /// [`ALL`](https://docs.oracle.com/en/java/javase/19/docs/api/java.logging/java/util/logging/Level.html#ALL)
  All,
  /// [`CONFIG`](https://docs.oracle.com/en/java/javase/19/docs/api/java.logging/java/util/logging/Level.html#CONFIG)
  Config,
  /// [`FINE`](https://docs.oracle.com/en/java/javase/19/docs/api/java.logging/java/util/logging/Level.html#FINE)
  Fine,
  /// [`FINER`](https://docs.oracle.com/en/java/javase/19/docs/api/java.logging/java/util/logging/Level.html#FINER)
  Finer,
  /// [`FINEST`](https://docs.oracle.com/en/java/javase/19/docs/api/java.logging/java/util/logging/Level.html#FINEST)
  Finest,
  /// [`INFO`](https://docs.oracle.com/en/java/javase/19/docs/api/java.logging/java/util/logging/Level.html#INFO)
  Info,
  /// [`OFF`](https://docs.oracle.com/en/java/javase/19/docs/api/java.logging/java/util/logging/Level.html#OFF)
  Off,
  /// [`WARNING`](https://docs.oracle.com/en/java/javase/19/docs/api/java.logging/java/util/logging/Level.html#WARNING)
  Warning,
  /// [`SEVERE`](https://docs.oracle.com/en/java/javase/19/docs/api/java.logging/java/util/logging/Level.html#SEVERE)
  Severe,
}

impl Level {
  /// ```
  /// use toad_jni::java::util::logging::Level;
  ///
  /// assert_eq!(Level::from_log_level(log::Level::Trace), Level::Finer);
  /// assert_eq!(Level::from_log_level(log::Level::Debug), Level::Fine);
  /// assert_eq!(Level::from_log_level(log::Level::Info), Level::Info);
  /// assert_eq!(Level::from_log_level(log::Level::Warn), Level::Warning);
  /// assert_eq!(Level::from_log_level(log::Level::Error), Level::Severe);
  /// ```
  pub fn from_log_level(lv: log::Level) -> Self {
    match lv {
      | log::Level::Error => Self::Severe,
      | log::Level::Warn => Self::Warning,
      | log::Level::Info => Self::Info,
      | log::Level::Debug => Self::Fine,
      | log::Level::Trace => Self::Finer,
    }
  }
}

impl java::Object for Level {
  fn upcast(e: &mut java::Env, jobj: java::lang::Object) -> Self {
    use Level::*;

    let all = All.downcast(e);
    let config = Config.downcast(e);
    let fine = Fine.downcast(e);
    let finer = Finer.downcast(e);
    let finest = Finest.downcast(e);
    let info = Info.downcast(e);
    let off = Off.downcast(e);
    let warning = Warning.downcast(e);
    let severe = Severe.downcast(e);

    if jobj.equals(e, &all) {
      Self::All
    } else if jobj.equals(e, &config) {
      Self::Config
    } else if jobj.equals(e, &fine) {
      Self::Fine
    } else if jobj.equals(e, &finer) {
      Self::Finer
    } else if jobj.equals(e, &finest) {
      Self::Finest
    } else if jobj.equals(e, &info) {
      Self::Info
    } else if jobj.equals(e, &off) {
      Self::Off
    } else if jobj.equals(e, &warning) {
      Self::Warning
    } else if jobj.equals(e, &severe) {
      Self::Severe
    } else {
      panic!("not java.util.logging.Level: {}", jobj.to_string(e));
    }
  }

  fn downcast(self, e: &mut java::Env) -> java::lang::Object {
    self.downcast_ref(e)
  }

  fn downcast_ref(&self, e: &mut java::Env) -> java::lang::Object {
    use Level::*;

    type F = java::StaticField<Level, NoUpcast<Level>>;

    struct Field {
      all: F,
      off: F,
      cfg: F,
      info: F,
      warning: F,
      severe: F,
      fine: F,
      finer: F,
      finest: F,
    }

    static FIELD: Field = Field { all: F::new("ALL"),
                                  off: F::new("OFF"),
                                  cfg: F::new("CONFIG"),
                                  info: F::new("INFO"),
                                  warning: F::new("WARNING"),
                                  severe: F::new("SEVERE"),
                                  fine: F::new("FINE"),
                                  finer: F::new("FINER"),
                                  finest: F::new("FINEST") };

    match self {
      | All => &FIELD.all,
      | Config => &FIELD.cfg,
      | Fine => &FIELD.fine,
      | Finer => &FIELD.finer,
      | Finest => &FIELD.finest,
      | Info => &FIELD.info,
      | Off => &FIELD.off,
      | Warning => &FIELD.warning,
      | Severe => &FIELD.severe,
    }.get(e)
     .object()
  }
}

impl java::Class for Level {
  const PATH: &'static str = "java/util/logging/Level";
}
