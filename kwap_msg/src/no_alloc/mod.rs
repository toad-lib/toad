use arrayvec::ArrayVec;

use crate::parsing::*;

#[doc(hidden)]
pub mod opt;

#[doc(inline)]
pub use opt::*;

#[doc = include_str!("../../docs/no_alloc/Message.md")]
#[cfg_attr(any(feature = "docs", docsrs), doc(cfg(feature = "alloc")))]
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
  pub opts: ArrayVec<opt::Opt<OPT_CAP>, N_OPTS>,
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
pub struct Payload<const PAYLOAD_CAP: usize>(pub ArrayVec<u8, PAYLOAD_CAP>);

/// Uniquely identifies a single message that may be retransmitted.
///
/// For a little more context and the difference between [`Id`] and [`Token`], see [`Token`].
///
/// See [RFC7252 - Message Details](https://datatracker.ietf.org/doc/html/rfc7252#section-3) for context
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct Id(pub u16);

impl<T: IntoIterator<Item = u8>> TryConsumeBytes<T> for Id {
  type Error = MessageParseError;
  fn try_consume_bytes(bytes: T) -> Result<Self, Self::Error> {
    let bytes = bytes.into_iter().take(2).collect::<ArrayVec<_, 2>>();
    bytes.into_inner()
         .map(|bs| u16::from_be_bytes(bs))
         .map(Id)
         .map_err(|_| MessageParseError::UnexpectedEndOfStream)
  }
}

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

impl From<u8> for Byte1 {
  fn from(b: u8) -> Self {
    let ver = b >> 6; // bits 0 & 1
    let ty = b >> 4 & 0b11; // bits 2 & 3
    let tkl = b & 0b1111u8; // last 4 bits

    Byte1 { ver: Version(ver),
            ty: Type(ty),
            tkl: TokenLength(tkl) }
  }
}

/// Version of the CoAP protocol that the message adheres to.
///
/// As far as this project is concerned, this will always be 1. (But will not _always_ be 1)
///
/// See [RFC7252 - Message Details](https://datatracker.ietf.org/doc/html/rfc7252#section-3) for context
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct Version(pub u8);

/// Message type:
/// - Confirmable; "Please let me know when you received this"
/// - Acknowledgement; "I got your message!"
/// - Non-confirmable; "I don't care if this gets to you"
/// - Reset; ""
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

/// # PANICS
/// Panics when iterator passed to this implementation contains > 8 bytes.
impl<T: IntoIterator<Item = u8>> TryConsumeBytes<T> for Token {
  type Error = MessageParseError;

  fn try_consume_bytes(bytes: T) -> Result<Self, Self::Error> {
    let bytes = bytes.into_iter().collect::<ArrayVec<_, 8>>();

    let mut array_u64: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 0];

    // pad the front with zeroes and copy values to array
    core::iter::repeat(0u8).take(8 - bytes.len())
                           .chain(bytes.into_iter())
                           .enumerate()
                           .for_each(|(ix, b)| {
                             array_u64[ix] = b;
                           });

    Ok(Token(u64::from_be_bytes(array_u64)))
  }
}

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

impl From<u8> for Code {
  fn from(b: u8) -> Self {
    let class = b >> 5;
    let detail = b & 0b0011111;

    Code { class, detail }
  }
}

impl<const PAYLOAD_CAP: usize, const N_OPTS: usize, const OPT_CAP: usize> TryFromBytes
  for Message<PAYLOAD_CAP, N_OPTS, OPT_CAP>
{
  type Error = MessageParseError;

  fn try_from_bytes<T: IntoIterator<Item = u8>>(bytes: T) -> Result<Self, Self::Error> {
    let mut bytes = bytes.into_iter();

    let Byte1 { tkl, ty, ver } = Self::Error::try_next(&mut bytes)?.into();

    if tkl.0 > 8 {
      Err(Self::Error::InvalidTokenLength(tkl.0 as u8))?;
    }

    let code: Code = Self::Error::try_next(&mut bytes)?.into();
    let id: Id = Id::try_consume_bytes(&mut bytes)?;
    let token = Token::try_consume_bytes(bytes.by_ref().take(tkl.0 as usize))?;
    let opts = ArrayVec::<Opt<OPT_CAP>, N_OPTS>::try_consume_bytes(&mut bytes).map_err(Self::Error::OptParseError)?;
    let mut payload_bytes = ArrayVec::new();
    bytes.try_for_each(|b| {
           payload_bytes.try_push(b)
                        .map_err(|_| Self::Error::PayloadTooLong(PAYLOAD_CAP))
         })?;

    let payload = Payload(payload_bytes);

    Ok(Message { tkl,
                 id,
                 ty,
                 ver,
                 code,
                 token,
                 opts,
                 payload })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_byte1() {
    let byte = 0b_01_10_0011u8;
    let byte = Byte1::from(byte);
    assert_eq!(byte,
               Byte1 { ver: Version(1),
                       ty: Type(2),
                       tkl: TokenLength(3) })
  }

  #[test]
  fn parse_id() {
    let id_bytes = 34u16.to_be_bytes();
    let id = Id::try_consume_bytes(id_bytes).unwrap();
    assert_eq!(id, Id(34));
  }

  #[test]
  fn parse_code() {
    let byte = 0b_01_000101u8;
    let code = Code::from(byte);
    assert_eq!(code, Code { class: 2, detail: 5 })
  }

  #[test]
  fn parse_token() {
    let valid_a: [u8; 1] = [0b_00000001u8];
    let valid_a = Token::try_consume_bytes(valid_a).unwrap();
    assert_eq!(valid_a, Token(1));

    let valid_b: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 1];
    let valid_b = Token::try_consume_bytes(valid_b).unwrap();
    assert_eq!(valid_a, valid_b);
  }
}
