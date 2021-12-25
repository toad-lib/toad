use tinyvec::ArrayVec;

use crate::parsing::*;

pub(crate) mod impl_from_bytes;
pub(crate) mod impl_get_size;
pub(crate) mod impl_to_bytes;

#[doc(hidden)]
pub mod opt;

#[doc(inline)]
pub use opt::*;

#[doc = include_str!("../../docs/no_alloc/Message.md")]
#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub struct Message<const PAYLOAD_CAP: usize, const N_OPTS: usize, const OPT_CAP: usize> {
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
  pub opts: ArrayVec<[opt::Opt<OPT_CAP>; N_OPTS]>,
  /// See [`Payload`]
  pub payload: Payload<PAYLOAD_CAP>,
}

/// Low-level representation of the message payload
///
/// Both requests and responses may include a payload, depending on the
/// Method or Response Code, respectively.
///
/// # Related
/// - [RFC7252#section-5.5 Payloads and Representations](https://datatracker.ietf.org/doc/html/rfc7252#section-5.5)
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct Payload<const PAYLOAD_CAP: usize>(pub ArrayVec<[u8; PAYLOAD_CAP]>);

/// Uniquely identifies a single message that may be retransmitted.
///
/// For a little more context and the difference between [`Id`] and [`Token`], see [`Token`].
///
/// See [RFC7252 - Message Details](https://datatracker.ietf.org/doc/html/rfc7252#section-3) for context
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct Id(pub u16);

/// Struct representing the first byte of a message.
///
/// ```text
/// CoAP version
/// |
/// |  Message type (request, response, empty)
/// |  |
/// |  |  Length of token, in bytes. (4-bit integer)
/// |  |  |
/// vv vv vvvv
/// 01 00 0000
/// ```
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub(crate) struct Byte1 {
  pub(crate) ver: Version,
  pub(crate) ty: Type,
  pub(crate) tkl: TokenLength,
}

/// Version of the CoAP protocol that the message adheres to.
///
/// As far as this project is concerned, this will always be 1. (But will not _always_ be 1)
///
/// See [RFC7252 - Message Details](https://datatracker.ietf.org/doc/html/rfc7252#section-3) for context
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct Version(pub u8);

impl Default for Version {
  fn default() -> Self {Version(1)}
}

/// Message type:
/// - 0 Confirmable; "Please let me know when you received this"
/// - 1 Acknowledgement; "I got your message!"
/// - 2 Non-confirmable; "I don't care if this gets to you"
/// - 3 Reset; ""
///
/// See [RFC7252 - Message Details](https://datatracker.ietf.org/doc/html/rfc7252#section-3) for context
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct Type(pub u8);

/// Length (in bytes) of the [`Token`]. Tokens are between 0 and 8 bytes in length.
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct TokenLength(pub u8);

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

/// Low-level representation of the code of a message.
/// Identifying it as a request or response
///
/// # Examples
/// ```
/// use kwap_msg::no_alloc::Code;
/// assert_eq!(Code { class: 2, detail: 5 }.to_string(), "2.05".to_string())
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
  /// or if the `alloc` feature of `kwap` is enabled there is a `ToString` implementation provided for Code.
  /// ```
  /// use kwap_msg::no_alloc::Code;
  ///
  /// let code = Code { class: 2, detail: 5 };
  /// let chars = code.to_human();
  /// let string = String::from_iter(chars);
  /// assert_eq!(string, "2.05".to_string());
  /// ```
  pub fn to_human(&self) -> [char; 4] {
    let to_char = |d: u8| char::from_digit(d.into(), 10).unwrap();
    [to_char(self.class),
     '.',
     to_char(self.detail / 10),
     to_char(self.detail % 10)]
  }
}

#[cfg(test)]
pub(self) fn test_msg() -> (Message<13, 1, 16>, Vec<u8>) {
  let header: [u8; 4] = 0b01_00_0001_01000101_0000000000000001u32.to_be_bytes();
  let token: [u8; 1] = [254u8];
  let content_format: &[u8] = b"application/json";
  let options: [&[u8]; 2] = [&[0b_1100_1101u8, 0b00000011u8], content_format];
  let payload: [&[u8]; 2] = [&[0b_11111111u8], b"hello, world!"];
  let bytes = [header.as_ref(),
               token.as_ref(),
               options.concat().as_ref(),
               payload.concat().as_ref()].concat();

  let mut opts = ArrayVec::new();
  let opt = Opt::<16> { delta: OptDelta(12),
                        value: OptValue(content_format.iter().copied().collect()) };
  opts.push(opt);

  let msg = Message::<13, 1, 16> { id: Id(1),
                                   ty: Type(0),
                                   ver: Version(1),
                                   token: Token(254),
                                   tkl: TokenLength(1),
                                   opts,
                                   code: Code { class: 2, detail: 5 },
                                   payload: Payload(b"hello, world!".into_iter().copied().collect()) };
  (msg, bytes)
}
