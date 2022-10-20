use tinyvec::ArrayVec;
use toad_common::Cursor;
use toad_macros::rfc_7252_doc;

use super::MessageParseError;
use crate::from_bytes::TryConsumeBytes;

#[doc = rfc_7252_doc!("5.3.1")]
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct Token(pub tinyvec::ArrayVec<[u8; 8]>);

impl Token {
  /// Take an arbitrary-length sequence of bytes and turn it into an opaque message token
  ///
  /// Currently uses the BLAKE2 hashing algorithm, but this may change in the future.
  ///
  /// ```
  /// use toad_msg::Token;
  ///
  /// let my_token = Token::opaque(&[0, 1, 2]);
  /// ```
  pub fn opaque(data: &[u8]) -> Token {
    use blake2::digest::consts::U8;
    use blake2::{Blake2b, Digest};

    let mut digest = Blake2b::<U8>::new();
    digest.update(data);
    Token(Into::<[u8; 8]>::into(digest.finalize()).into())
  }
}
