use crate::java::{self, NoUpcast, Object};

/// `java.net.ProtocolFamily`
pub struct ProtocolFamily(java::lang::Object);
java::object_newtype!(ProtocolFamily);
impl java::Class for ProtocolFamily {
  const PATH: &'static str = "java/net/ProtocolFamily";
}

/// `java.net.StandardProtocolFamily`
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StandardProtocolFamily {
  /// `java.net.StandardProtocolFamily.INET`
  INet,
  /// `java.net.StandardProtocolFamily.INET6`
  INet6,
  /// `java.net.StandardProtocolFamily.UNIX`
  Unix,
}

impl StandardProtocolFamily {
  /// cast a `StandardProtocolFamily` to [`ProtocolFamily`]
  pub fn into_protocol_family(self, e: &mut java::Env) -> ProtocolFamily {
    ProtocolFamily(self.downcast(e))
  }
}

impl java::Object for StandardProtocolFamily {
  fn upcast(e: &mut java::Env, jobj: java::lang::Object) -> Self {
    let (inet, inet6, unix) =
      (Self::INet.downcast(e), Self::INet6.downcast(e), Self::Unix.downcast(e));
    if jobj.equals(e, &inet) {
      Self::INet
    } else if jobj.equals(e, &inet6) {
      Self::INet6
    } else if jobj.equals(e, &unix) {
      Self::Unix
    } else {
      panic!("not StandardProtocolFamily: {}", jobj.to_string(e));
    }
  }

  fn downcast(self, e: &mut java::Env) -> java::lang::Object {
    static INET: java::StaticField<StandardProtocolFamily, NoUpcast<StandardProtocolFamily>> =
      java::StaticField::new("INET");
    static INET6: java::StaticField<StandardProtocolFamily, NoUpcast<StandardProtocolFamily>> =
      java::StaticField::new("INET6");
    static UNIX: java::StaticField<StandardProtocolFamily, NoUpcast<StandardProtocolFamily>> =
      java::StaticField::new("UNIX");

    match self {
      | Self::INet => INET.get(e).downcast(e),
      | Self::INet6 => INET6.get(e).downcast(e),
      | Self::Unix => UNIX.get(e).downcast(e),
    }
  }

  fn downcast_ref(&self, e: &mut java::Env) -> java::lang::Object {
    self.downcast(e)
  }
}

impl java::Class for StandardProtocolFamily {
  const PATH: &'static str = "java/net/StandardProtocolFamily";
}
