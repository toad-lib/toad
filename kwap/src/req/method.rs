use kwap_msg::Code;

use crate::code;

/// Request method
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct Method(pub(super) Code);

#[cfg(not(feature = "no_std"))]
impl std::fmt::Display for Method {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let string = match self.0 {
      | Code { class: 0, detail: 0 } => "EMPTY".to_string(),
      | Code { class: 0, detail: 1 } => "GET".to_string(),
      | Code { class: 0, detail: 2 } => "PUT".to_string(),
      | Code { class: 0, detail: 3 } => "POST".to_string(),
      | Code { class: 0, detail: 4 } => "DELETE".to_string(),
      | c => c.to_string(),
    };

    write!(f, "{}", string)
  }
}

impl Method {
  code!(rfc7252("4.1")   EMPTY  = Method(0 . 00));
  code!(rfc7252("5.8.1") GET    = Method(0 . 01));
  code!(rfc7252("5.8.2") POST   = Method(0 . 02));
  code!(rfc7252("5.8.3") PUT    = Method(0 . 03));
  code!(rfc7252("5.8.4") DELETE = Method(0 . 04));
}
