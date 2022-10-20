use toad_common::{AppendCopy, Array, Cursor, GetSize};
use toad_macros::rfc_7252_doc;

/// Message Code
pub mod code;

/// Message parsing errors
pub mod parse_error;

/// Message ID
pub mod id;

/// Message Options
pub mod opt;

/// Message Type
pub mod ty;

/// Message Token
pub mod token;

/// Message Version
pub mod ver;

pub use code::*;
pub use id::*;
pub use opt::*;
pub use parse_error::*;
pub use token::*;
pub use ty::*;
pub use ver::*;

use crate::from_bytes::TryConsumeBytes;
use crate::TryFromBytes;

#[doc = rfc_7252_doc!("5.5")]
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct Payload<C>(pub C);

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
  pub(crate) tkl: u8,
}

impl TryFrom<u8> for Byte1 {
  type Error = MessageParseError;

  fn try_from(b: u8) -> Result<Self, Self::Error> {
    let ver = b >> 6; // bits 0 & 1
    let ty = b >> 4 & 0b11; // bits 2 & 3
    let tkl = b & 0b1111u8; // last 4 bits

    Ok(Byte1 { ver: Version(ver),
               ty: Type::try_from(ty)?,
               tkl })
  }
}

impl<PayloadBytes: Array<Item = u8>,
      OptionValue: Array<Item = u8> + AppendCopy<u8>,
      Options: Array<Item = Opt<OptionValue>>> GetSize for Message<PayloadBytes, Options>
{
  fn get_size(&self) -> usize {
    let header_size = 4;
    let payload_marker_size = 1;
    let payload_size = self.payload.0.get_size();
    let token_size = self.token.0.len();
    let opts_size: usize = self.opts.iter().map(|o| o.get_size()).sum();

    header_size + payload_marker_size + payload_size + token_size + opts_size
  }

  fn max_size(&self) -> Option<usize> {
    None
  }

  fn is_full(&self) -> bool {
    false
  }
}

/// # `Message` struct
/// Low-level representation of a message that has been parsed from the raw binary format.
///
/// Note that `Message` is generic over 3 [`Array`]s:
///  - `PayloadC`: the byte buffer used to store the message's [`Payload`]
///  - `OptC`: byte buffer used to store [`Opt`]ion values ([`OptValue`])
///  - `Opts`: collection of [`Opt`]ions in the message
///
/// Messages support both serializing to bytes and from bytes, by using the provided [`TryFromBytes`] and [`TryIntoBytes`] traits.
///
/// <details>
/// <summary><b>RFC7252 - CoAP Messaging Model</b></summary>
#[doc = concat!("\n#", rfc_7252_doc!("2.1"))]
/// </details>
/// <details>
/// <summary><b>RFC7252 - CoAP Message Binary Format</b></summary>
#[doc = concat!("\n#", rfc_7252_doc!("3"))]
/// </details>
///
/// ```
/// use toad_msg::TryFromBytes;
/// use toad_msg::*;
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
///
/// // `VecMessage` uses `Vec` as the backing structure for byte buffers
/// let msg = VecMessage::try_from_bytes(packet.clone()).unwrap();
/// # let opt = Opt {
/// #   delta: OptDelta(12),
/// #   value: OptValue(content_format.iter().map(|u| *u).collect()),
/// # };
/// let mut opts_expected = /* create expected options */
/// # Vec::new();
/// # opts_expected.push(opt);
///
/// let expected = VecMessage {
///   id: Id(1),
///   ty: Type::Con,
///   ver: Version(1),
///   token: Token(tinyvec::array_vec!([u8; 8] => 254)),
///   opts: opts_expected,
///   code: Code {class: 2, detail: 5},
///   payload: Payload(b"hello, world!".to_vec()),
/// };
///
/// assert_eq!(msg, expected);
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub struct Message<PayloadBytes, Options> {
  /// see [`Id`] for details
  pub id: Id,
  /// see [`Type`] for details
  pub ty: Type,
  /// see [`Version`] for details
  pub ver: Version,
  /// see [`Token`] for details
  pub token: Token,
  /// see [`Code`] for details
  pub code: Code,
  /// see [`opt::Opt`] for details
  pub opts: Options,
  /// see [`Payload`]
  pub payload: Payload<PayloadBytes>,
}

impl<PayloadBytes: Array<Item = u8> + AppendCopy<u8>,
      OptionValue: Array<Item = u8> + AppendCopy<u8> + 'static,
      Options: Array<Item = Opt<OptionValue>>> Message<PayloadBytes, Options>
{
  /// Create a new message that ACKs this one.
  ///
  /// This needs an [`Id`] to assign to the newly created message.
  ///
  /// ```
  /// // we are a server
  ///
  /// use std::net::SocketAddr;
  ///
  /// use toad_msg::{Id, VecMessage as Message};
  ///
  /// fn server_get_request() -> Option<(SocketAddr, Message)> {
  ///   // Servery sockety things...
  ///   # use std::net::{Ipv4Addr, ToSocketAddrs};
  ///   # use toad_msg::{Type, Code, Token, Version, Payload};
  ///   # let addr = (Ipv4Addr::new(0, 0, 0, 0), 1234);
  ///   # let addr = addr.to_socket_addrs().unwrap().next().unwrap();
  ///   # let msg = Message { code: Code::new(0, 0),
  ///   #                     id: Id(1),
  ///   #                     ty: Type::Con,
  ///   #                     ver: Version(1),
  ///   #                     token: Token(tinyvec::array_vec!([u8; 8] => 254)),
  ///   #                     opts: vec![],
  ///   #                     payload: Payload(vec![]) };
  ///   # Some((addr, msg))
  /// }
  ///
  /// fn server_send_msg(addr: SocketAddr, msg: Message) -> Result<(), ()> {
  ///   // Message sendy bits...
  ///   # Ok(())
  /// }
  ///
  /// let (addr, req) = server_get_request().unwrap();
  /// let ack_id = Id(req.id.0 + 1);
  /// let ack = req.ack(ack_id);
  ///
  /// server_send_msg(addr, ack).unwrap();
  /// ```
  pub fn ack(&self, id: Id) -> Self {
    Self { id,
           token: self.token,
           ver: Default::default(),
           ty: Type::Ack,
           code: Code::new(0, 0),
           payload: Payload(Default::default()),
           opts: Default::default() }
  }
}

impl<Bytes: AsRef<[u8]>,
      PayloadBytes: Array<Item = u8> + AppendCopy<u8>,
      OptionValue: Array<Item = u8> + AppendCopy<u8>,
      Options: Array<Item = Opt<OptionValue>>> TryFromBytes<Bytes>
  for Message<PayloadBytes, Options>
{
  type Error = MessageParseError;

  fn try_from_bytes(bytes: Bytes) -> Result<Self, Self::Error> {
    let mut bytes = Cursor::new(bytes);

    let Byte1 { tkl, ty, ver } = bytes.next()
                                      .ok_or_else(|| MessageParseError::eof())?
                                      .try_into()?;

    if tkl > 8 {
      return Err(Self::Error::InvalidTokenLength(tkl));
    }

    let code: Code = bytes.next().ok_or_else(|| MessageParseError::eof())?.into();
    let id: Id = Id::try_consume_bytes(&mut bytes)?;

    let token = bytes.take_exact(tkl as usize)
                     .ok_or_else(|| MessageParseError::eof())?;
    let token = tinyvec::ArrayVec::<[u8; 8]>::try_from(token).expect("tkl was checked to be <= 8");
    let token = Token(token);

    let opts = Options::try_consume_bytes(&mut bytes).map_err(Self::Error::OptParseError)?;

    let mut payload = PayloadBytes::reserve(bytes.remaining());
    payload.append_copy(bytes.take_until_end());
    let payload = Payload(payload);

    Ok(Message { id,
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
  use crate::VecMessage;

  #[test]
  fn parse_msg() {
    let (expect, msg) = crate::test_msg();
    assert_eq!(VecMessage::try_from_bytes(&msg).unwrap(), expect)
  }

  #[test]
  fn parse_byte1() {
    let byte = 0b_01_10_0011u8;
    let byte = Byte1::try_from(byte).unwrap();
    assert_eq!(byte,
               Byte1 { ver: Version(1),
                       ty: Type::Ack,
                       tkl: 3 })
  }

  #[test]
  fn parse_id() {
    let mut id_bytes = Cursor::new(34u16.to_be_bytes());
    let id = Id::try_consume_bytes(&mut id_bytes).unwrap();
    assert_eq!(id, Id(34));
  }
}
