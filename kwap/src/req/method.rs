use kwap_msg::Code;

use crate::code;

/// Request method
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct Method(pub(super) Code);

impl Method {
  code!(rfc7252("5.8.1") GET    = Method(0 . 01));
  code!(rfc7252("5.8.2") POST   = Method(0 . 02));
  code!(rfc7252("5.8.3") PUT    = Method(0 . 03));
  code!(rfc7252("5.8.4") DELETE = Method(0 . 04));
}
