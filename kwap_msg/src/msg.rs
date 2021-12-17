#[allow(unused_imports)]
use alloc::string::{String, ToString};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

#[cfg(not(feature = "alloc"))]
use arrayvec::ArrayVec;

/// Serializing and Deserializing from bytes
pub mod bytes;
pub use bytes::*;

/// Opt and related data structures
pub mod opt;

/// Uniquely identifies a single message that may be retransmitted.
///
/// For a little more context and the difference between [`Id`] and [`Token`], see [`Token`].
///
/// See [RFC7252 - Message Details](https://datatracker.ietf.org/doc/html/rfc7252#section-3) for context
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct Id(pub u16);

/// Message type:
/// - Confirmable; "Please let me know when you received this"
/// - Acknowledgement; "I got your message!"
/// - Non-confirmable; "I don't care if this gets to you"
/// - Reset; ""
///
/// See [RFC7252 - Message Details](https://datatracker.ietf.org/doc/html/rfc7252#section-3) for context
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct Type(pub u8);

/// Version of the CoAP protocol that the message adheres to.
///
/// As far as this project is concerned, this will always be 1. (But will not _always_ be 1)
///
/// See [RFC7252 - Message Details](https://datatracker.ietf.org/doc/html/rfc7252#section-3) for context
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct Version(pub u8);

/// Message token for matching requests to responses
///
/// Note that this is different from [`Id`],
/// which uniquely identifies a message that may be retransmitted.
///
/// For example, Client may send a confirmable message with id 1 and token 321
/// to Server multiple times,
/// then Server confirms and sends a response
/// with a different id (because it's a different message),
/// but token 321 (so the client knows which request the response is responding to)
///
/// Note that the format of the token is not necessarily an integer according to
/// the coap spec, but is interpreted by this library as an 8 byte unsigned integer in network byte order.
///
/// See [RFC7252 - Message Details](https://datatracker.ietf.org/doc/html/rfc7252#section-3) for context
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct Token(pub u64);

/// Length (in bytes) of the [`Token`]. Tokens are between 0 and 8 bytes in length.
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct TokenLength(pub u8);

/// Low-level representation of the code of a message.
/// Identifying it as a request or response
///
/// # Examples
/// ```
/// assert_eq!(kwap_msg::Code {class: 2, detail: 5}.to_string(), "2.05".to_string())
/// ```
///
/// See [RFC7252 - Message Details](https://datatracker.ietf.org/doc/html/rfc7252#section-3) for context
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct Code {
  /// The "class" of message codes identify it as a request or response, and provides the class of response status:
  ///
  /// |class|meaning|
  /// |---|---|
  /// |`0`|Message is a request|
  /// |`2`|Message is a success response|
  /// |`4`|Message is a client error response|
  /// |`5`|Message is a server error response|
  pub class: u8,

  /// 2-digit integer (range `[0, 32)`) that provides granular information about the response status.
  ///
  /// Will always be `0` for requests.
  pub detail: u8,
}

impl Code {
  /// Get the human string representation of a message code
  ///
  /// # Returns
  /// A `char` array
  ///
  /// This is to avoid unnecessary heap allocation,
  /// you can create a `String` with `FromIterator::<String>::from_iter`,
  /// or if the `alloc` feature of `kwap` is enabled there is a [`ToString`] implementation provided for Code.
  /// ```
  /// use kwap_msg::Code;
  ///
  /// let code = Code {class: 2, detail: 5};
  /// let chars = code.to_human();
  /// let string = String::from_iter(chars);
  /// assert_eq!(string, "2.05".to_string());
  /// ```
  pub fn to_human(&self) -> [char; 4] {
    let to_char = |d: u8| char::from_digit(d.into(), 10).unwrap();
    [to_char(self.class), '.', to_char(self.detail / 10), to_char(self.detail % 10)]
  }
}

#[cfg(feature = "alloc")]
#[cfg_attr(any(feature = "docs", docsrs), doc(cfg(feature = "alloc")))]
impl ToString for Code {
  fn to_string(&self) -> String {
    String::from_iter(self.to_human())
  }
}

/// Low-level representation of the message payload
///
/// Both requests and responses may include a payload, depending on the
/// Method or Response Code, respectively.
///
/// # Related
/// - [RFC7252#section-5.5 Payloads and Representations](https://datatracker.ietf.org/doc/html/rfc7252#section-5.5)
#[cfg(feature = "alloc")]
#[cfg_attr(any(feature = "docs", docsrs), doc(cfg(feature = "alloc")))]
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct Payload(pub Vec<u8>);

/// Low-level representation of a message
/// that has been parsed from a byte array
///
/// To convert an iterator over bytes into a Message, use the provided [`FromIterator`] implementation.
///
/// TODO: make heapless version
///
/// ```
/// use kwap_msg::*;
///
/// # //                       version  token len  code (2.05 Content)
/// # //                       |        |          /
/// # //                       |  type  |         /  message ID
/// # //                       |  |     |        |   |
/// # //                       vv vv vvvv vvvvvvvv vvvvvvvvvvvvvvvv
/// # let header: [u8; 4] = 0b_01_00_0001_01000101_0000000000000001u32.to_be_bytes();
/// # let token: [u8; 1] = [254u8];
/// # let content_format: &[u8] = b"application/json";
/// # let options: [&[u8]; 2] = [&[0b_1100_1101u8, 0b00000011u8], content_format];
/// # let payload: [&[u8]; 2] = [&[0b_11111111u8], b"hello, world!"];
/// let packet: Vec<u8> = /* bytes! */
/// # [header.as_ref(), token.as_ref(), options.concat().as_ref(), payload.concat().as_ref()].concat();
/// let msg = Message::try_consume_bytes(&packet).unwrap();
/// # let opt = opt::Opt {
/// #   delta: opt::Delta(12),
/// #   value: opt::Value {len: 16, data: content_format.iter().map(|u| *u).collect()}
/// # };
/// let mut opts_expected = /* create expected options */
/// # Vec::new();
/// # opts_expected.push(opt);
///
/// let expected = Message {
///   id: Id(1),
///   ty: Type(0),
///   ver: Version(1),
///   token: Token(254),
///   tkl: TokenLength(1),
///   opts: opts_expected,
///   code: Code {class: 2, detail: 5},
///   payload: Payload(b"hello, world!".to_vec()),
/// };
///
/// assert_eq!(msg, expected);
/// ```
///
/// See [RFC7252 - Message Details](https://datatracker.ietf.org/doc/html/rfc7252#section-3) for context
#[cfg(feature = "alloc")]
#[cfg_attr(any(feature = "docs", docsrs), doc(cfg(feature = "alloc")))]
#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub struct Message {
  /// see [`Id`] for details
  pub id: Id,
  /// see [`Type`] for details
  pub ty: Type,
  /// see [`Version`] for details
  pub ver: Version,
  /// see [`TokenLength`] for details
  pub tkl: TokenLength,
  /// see [`Token`] for details
  pub token: Token,
  /// see [`Code`] for details
  pub code: Code,
  /// see [`opt::Opt`] for details
  pub opts: Vec<opt::Opt>,
  /// See [`Payload`]
  pub payload: Payload,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn try_consume_bytes() {
    //                       version  token len  code (2.05 Content)
    //                       |        |          /
    //                       |  type  |         /  message ID
    //                       |  |     |        |   |
    //                       vv vv vvvv vvvvvvvv vvvvvvvvvvvvvvvv
    let header: [u8; 4] = 0b_01_00_0001_01000101_0000000000000001u32.to_be_bytes();
    let token: [u8; 1] = [254u8];
    let content_format: &[u8] = b"application/json";
    let options: [&[u8]; 2] = [&[0b_1100_1101u8, 0b00000011u8], content_format];
    let payload: [&[u8]; 2] = [&[0b_11111111u8], b"hello, world!"];
    let packet: Vec<u8> = [header.as_ref(), token.as_ref(), options.concat().as_ref(), payload.concat().as_ref()].concat();
    let msg = Message::try_consume_bytes(&packet).unwrap();
    let opt = opt::Opt {delta: opt::Delta(12), value: opt::Value {len: 16, data: content_format.iter().map(|u| *u).collect()}};
    let mut opts_expected = Vec::new();
    opts_expected.push(opt);
    let expected = Message {
      id: Id(1),
      ty: Type(0),
      ver: Version(1),
      token: Token(254),
      tkl: TokenLength(1),
      opts: opts_expected,
      code: Code {class: 2, detail: 5},
      payload: Payload(b"hello, world!".to_vec()),
    };
    assert_eq!(msg, expected);
  }
}
