use std_alloc::{string::{String, ToString},
                vec::Vec};

pub use crate::no_alloc::{Code, Id, Token, TokenLength, Type, Version};
use crate::{from_bytes::*, no_alloc::Byte1};

#[doc(hidden)]
pub mod opt;

#[doc(inline)]
pub use opt::*;

mod impl_get_size;
mod impl_to_bytes;

/// Low-level representation of the message payload
///
/// Both requests and responses may include a payload, depending on the
/// Method or Response Code, respectively.
///
/// # Related
/// - [RFC7252#section-5.5 Payloads and Representations](https://datatracker.ietf.org/doc/html/rfc7252#section-5.5)
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct Payload(pub Vec<u8>);

#[doc = include_str!("../../docs/no_alloc/Message.md")]
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

impl<'a> TryFromBytes<&'a u8> for Message {
  type Error = MessageParseError;

  fn try_from_bytes<I: IntoIterator<Item = &'a u8>>(bytes: I) -> Result<Self, Self::Error> {
    Self::try_from_bytes(bytes.into_iter().copied())
  }
}

impl TryFromBytes<u8> for Message {
  type Error = MessageParseError;

  fn try_from_bytes<I: IntoIterator<Item = u8>>(bytes: I) -> Result<Self, Self::Error> {
    let mut bytes = bytes.into_iter();

    let Byte1 { tkl, ty, ver } = Self::Error::try_next(&mut bytes)?.into();

    if tkl.0 > 8 {
      return Err(Self::Error::InvalidTokenLength(tkl.0 as u8));
    }

    let code: Code = Self::Error::try_next(&mut bytes)?.into();
    let id: Id = Id::try_consume_bytes(&mut bytes)?;
    let token = Token::try_consume_bytes(&mut bytes.by_ref().take(tkl.0 as usize))?;
    let opts = Vec::<Opt>::try_consume_bytes(&mut bytes).map_err(Self::Error::OptParseError)?;
    let payload = Payload(bytes.collect());

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

impl ToString for Code {
  fn to_string(&self) -> String {
    String::from_iter(self.to_human())
  }
}

#[cfg(test)]
pub(self) fn test_msg() -> (Message, Vec<u8>) {
  let header: [u8; 4] = 0b01_00_0001_01000101_0000000000000001u32.to_be_bytes();
  let token: [u8; 1] = [254u8];
  let content_format: &[u8] = b"application/json";
  let options: [&[u8]; 2] = [&[0b_1100_1101u8, 0b00000011u8], content_format];
  let payload: [&[u8]; 2] = [&[0b_11111111u8], b"hello, world!"];
  let bytes = [header.as_ref(),
               token.as_ref(),
               options.concat().as_ref(),
               payload.concat().as_ref()].concat();

  let mut opts = Vec::new();
  let opt = Opt { delta: OptDelta(12),
                  value: OptValue(content_format.iter().copied().collect()) };
  opts.push(opt);

  let msg = Message { id: Id(1),
                      ty: Type(0),
                      ver: Version(1),
                      token: Token(254),
                      tkl: TokenLength(1),
                      opts,
                      code: Code { class: 2, detail: 5 },
                      payload: Payload(b"hello, world!".into_iter().copied().collect()) };
  (msg, bytes)
}
