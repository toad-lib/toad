use std_alloc::vec::Vec;

pub use crate::no_alloc::opt::{EnumerateOptNumbers, EnumerateOptNumbersIter, GetOptDelta, OptDelta, OptNumber};
use crate::parsing::*;

#[doc = include_str!("../../docs/no_alloc/opt/Opt.md")]
#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub struct Opt {
  /// See [`OptDelta`]
  pub delta: OptDelta,
  /// See [`OptValue`]
  pub value: OptValue,
}

impl GetOptDelta for Opt {
  fn get_delta(&self) -> OptDelta {
    self.delta
  }
}

/// Option Value
///
/// # Related
/// - [RFC7252#section-3.1 Option Format](https://datatracker.ietf.org/doc/html/rfc7252#section-3.1)
/// - [RFC7252#section-5.4 Options](https://datatracker.ietf.org/doc/html/rfc7252#section-5.4)
#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub struct OptValue(pub Vec<u8>);

impl<T: IntoIterator<Item = u8>> TryConsumeBytes<T> for Vec<Opt> {
  type Error = OptParseError;

  fn try_consume_bytes(bytes: T) -> Result<Self, Self::Error> {
    let mut bytes = bytes.into_iter();
    let mut opts = Vec::new();

    loop {
      match Opt::try_consume_bytes(bytes.by_ref()) {
        | Ok(opt) => {
          opts.push(opt);
        },
        | Err(OptParseError::OptionsExhausted) => break Ok(opts),
        | Err(e) => break Err(e),
      }
    }
  }
}
impl<T: IntoIterator<Item = u8>> TryConsumeBytes<T> for Opt {
  type Error = OptParseError;

  fn try_consume_bytes(bytes: T) -> Result<Self, Self::Error> {
    let (opt_header, mut bytes) = opt_header(bytes)?;

    // NOTE: Delta **MUST** be consumed before Value. see comment on `opt_len_or_delta` for more info
    let delta = OptDelta::try_consume_bytes(&mut bytes)?;
    let value = OptValue::try_consume_bytes(&mut [opt_header].into_iter().chain(bytes))?;
    Ok(Opt { delta, value })
  }
}

impl<T: IntoIterator<Item = u8>> TryConsumeBytes<T> for OptValue {
  type Error = OptParseError;

  fn try_consume_bytes(bytes: T) -> Result<Self, Self::Error> {
    let mut bytes = bytes.into_iter();
    let first_byte = Self::Error::try_next(&mut bytes)?;
    let len = first_byte & 0b00001111;
    let len = opt_len_or_delta(len, &mut bytes, OptParseError::ValueLengthReservedValue(15))?;

    let data: Vec<u8> = bytes.take(len as usize).collect();
    if data.len() < len as usize {
      Err(OptParseError::UnexpectedEndOfStream)
    } else {
      Ok(OptValue(data))
    }
  }
}
