//! Builder for Java VM method signatures

/// Builder for JVM method signatures
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Sig {
  bytes: [u8; 256],
  len: usize,
  finished: bool,
}

impl Sig {
  /// `void`
  pub const VOID: Sig = Sig::empty().push_str("V");
  /// `byte`
  pub const BYTE: Sig = Sig::empty().push_str("B");
  /// `bool`
  pub const BOOL: Sig = Sig::empty().push_str("Z");
  /// `char`
  pub const CHAR: Sig = Sig::empty().push_str("C");
  /// `short`
  pub const SHORT: Sig = Sig::empty().push_str("S");
  /// `int`
  pub const INT: Sig = Sig::empty().push_str("I");
  /// `long`
  pub const LONG: Sig = Sig::empty().push_str("J");
  /// `float`
  pub const FLOAT: Sig = Sig::empty().push_str("F");
  /// `double`
  pub const DOUBLE: Sig = Sig::empty().push_str("D");
  const CLASS_PATH_OPEN: Sig = Sig::empty().push_str("L");
  const CLASS_PATH_CLOSE: Sig = Sig::empty().push_str(";");
  const ARRAY_OF: Sig = Sig::empty().push_str("[");
  const ARGS_OPEN: Sig = Sig::empty().push_str("(");
  const ARGS_CLOSE: Sig = Sig::empty().push_str(")");

  const fn empty() -> Self {
    Sig { bytes: [0; 256],
          len: 0,
          finished: false }
  }

  /// Create a new [`Sig`]
  pub const fn new() -> Self {
    Self::empty().concat(Self::ARGS_OPEN)
  }

  /// Type [`Sig`]nature for an array of other types
  pub const fn array_of(t: Self) -> Self {
    Self::empty().concat(Self::ARRAY_OF).concat(t)
  }

  /// Type [`Sig`]nature for a fully qualified class
  ///
  /// ```
  /// use toad_jni::Sig;
  ///
  /// assert_eq!(Sig::class("java/lang/Byte").as_str(), "Ljava/lang/Byte;");
  /// ```
  pub const fn class(path: &'static str) -> Self {
    Self::empty().concat(Self::CLASS_PATH_OPEN)
                 .push_str(path)
                 .concat(Self::CLASS_PATH_CLOSE)
  }

  const fn next(&self) -> usize {
    self.len
  }

  const fn concat(mut self, other: Self) -> Self {
    let mut i = 0;
    loop {
      if i == other.len {
        break;
      }
      self = self.push_byte(other.bytes[i]);
      i += 1;
    }

    self
  }

  const fn push_byte(mut self, b: u8) -> Self {
    if self.finished {
      panic!("cannot modify Sig after invoking .returning()")
    }
    let n = self.next();
    self.bytes[n] = b;
    self.len += 1;
    self
  }

  const fn push_str(mut self, s: &'static str) -> Self {
    let mut i = 0;
    loop {
      if i == s.len() {
        break;
      }

      let b = s.as_bytes()[i];
      self = self.push_byte(b);

      i += 1;
    }

    self
  }

  /// Add an argument to this method signature
  ///
  /// ```
  /// use toad_jni::Sig;
  ///
  /// assert_eq!(Sig::new().arg(Sig::BYTE)
  ///                      .arg(Sig::class("java/lang/Byte"))
  ///                      .returning(Sig::VOID)
  ///                      .as_str(),
  ///            "(BLjava/lang/Byte;)V");
  /// ```
  pub const fn arg(self, s: Sig) -> Self {
    self.concat(s)
  }

  /// Finalize the builder with the return type of the method
  ///
  /// ```
  /// use toad_jni::Sig;
  ///
  /// assert_eq!(Sig::new().arg(Sig::BYTE)
  ///                      .arg(Sig::class("java/lang/Byte"))
  ///                      .returning(Sig::VOID)
  ///                      .as_str(),
  ///            "(BLjava/lang/Byte;)V");
  /// ```
  pub const fn returning(mut self, s: Sig) -> Self {
    self = self.concat(Self::ARGS_CLOSE).concat(s);
    self.finished = true;
    self
  }

  /// Convert a [`Sig`] reference to [`str`]
  pub fn as_str(&self) -> &str {
    match core::str::from_utf8(&self.bytes[0..self.len]) {
      | Ok(s) => s,
      | _ => unreachable!(),
    }
  }
}

impl AsRef<str> for Sig {
  fn as_ref(&self) -> &str {
    self.as_str()
  }
}
