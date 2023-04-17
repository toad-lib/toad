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

    let (all, cfg, fine, finer, finest, info, off, warn, sev) = (All.downcast(e),
                                                                 Config.downcast(e),
                                                                 Fine.downcast(e),
                                                                 Finer.downcast(e),
                                                                 Finest.downcast(e),
                                                                 Info.downcast(e),
                                                                 Off.downcast(e),
                                                                 Warning.downcast(e),
                                                                 Severe.downcast(e));

    if jobj.equals(e, &all) {
      Self::All
    } else if jobj.equals(e, &cfg) {
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
    } else if jobj.equals(e, &warn) {
      Self::Warning
    } else if jobj.equals(e, &sev) {
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

    struct Field {
      all: java::StaticField<Level, NoUpcast<Level>>,
      off: java::StaticField<Level, NoUpcast<Level>>,
      cfg: java::StaticField<Level, NoUpcast<Level>>,
      info: java::StaticField<Level, NoUpcast<Level>>,
      warning: java::StaticField<Level, NoUpcast<Level>>,
      severe: java::StaticField<Level, NoUpcast<Level>>,
      fine: java::StaticField<Level, NoUpcast<Level>>,
      finer: java::StaticField<Level, NoUpcast<Level>>,
      finest: java::StaticField<Level, NoUpcast<Level>>,
    }

    static FIELD: Field = Field { all: java::StaticField::new("ALL"),
                                  off: java::StaticField::new("OFF"),
                                  cfg: java::StaticField::new("CONFIG"),
                                  info: java::StaticField::new("INFO"),
                                  warning: java::StaticField::new("WARNING"),
                                  severe: java::StaticField::new("SEVERE"),
                                  fine: java::StaticField::new("FINE"),
                                  finer: java::StaticField::new("FINER"),
                                  finest: java::StaticField::new("FINEST") };

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
