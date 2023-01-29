#[allow(unused_imports)]
use crate::Token;
use toad_common::Cursor;

use super::MessageParseError;
use crate::from_bytes::TryConsumeBytes;

/// # Message ID
///
/// 16-bit unsigned integer in network byte order.  Used to
/// detect message duplication and to match messages of type
/// Acknowledgement/Reset to messages of type Confirmable/Non-
/// confirmable.  The rules for generating a Message ID and matching
/// messages are defined in RFC7252 Section 4
///
/// For a little more context and the difference between [`Id`] and [`Token`], see [`Token`].
///
/// See [RFC7252 - Message Details](https://datatracker.ietf.org/doc/html/rfc7252#section-3) for context
#[derive(Copy, Clone, Hash, PartialEq, PartialOrd, Debug, Eq, Ord)]
pub struct Id(pub u16);

impl Id {
  /// Create an Id from a big-endian 2-byte unsigned int
  pub fn from_be_bytes(bs: [u8; 2]) -> Self {
    Self(u16::from_be_bytes(bs))
  }
}

impl<Bytes: AsRef<[u8]>> TryConsumeBytes<Bytes> for Id {
  type Error = MessageParseError;

  fn try_consume_bytes(bytes: &mut Cursor<Bytes>) -> Result<Self, Self::Error> {
    match bytes.take_exact(2) {
      | Some(&[a, b]) => Ok(Id::from_be_bytes([a, b])),
      | _ => Err(MessageParseError::eof()),
    }
  }
}
