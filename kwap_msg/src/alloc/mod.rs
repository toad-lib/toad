use std_alloc::{vec::Vec, string::{String, ToString}};

use crate::*;
use crate::parsing::*;
use crate::no_alloc::Byte1;
pub use crate::no_alloc::{Code, Id, Type, Token, TokenLength, Version};

#[doc(hidden)]
pub mod opt;

#[doc(inline)]
pub use opt::*;

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

impl TryFromBytes for Message {
  type Error = MessageParseError;

  fn try_from_bytes<T: IntoIterator<Item = u8>>(bytes: T) -> Result<Self, Self::Error> {
    let mut bytes = bytes.into_iter();

    let Byte1 {tkl, ty, ver} = Self::Error::try_next(&mut bytes)?.into();

    if tkl.0 > 8 {
      Err(Self::Error::InvalidTokenLength(tkl.0 as u8))?;
    }

    let code: Code = Self::Error::try_next(&mut bytes)?.into();
    let id: Id = Id::try_consume_bytes(&mut bytes)?;
    let token = Token::try_consume_bytes(bytes.by_ref().take(tkl.0 as usize))?;
    let opts = Vec::<Opt>::try_consume_bytes(&mut bytes).map_err(Self::Error::OptParseError)?;
    let payload = Payload(bytes.collect());

    Ok(Message {tkl, id, ty, ver, code, token, opts, payload})
  }
}

impl ToString for Code {
  fn to_string(&self) -> String {
    String::from_iter(self.to_human())
  }
}
