use std_alloc::vec::Vec;

pub use crate::no_alloc::opt::{EnumerateOptNumbers, EnumerateOptNumbersIter, GetOptDelta, OptDelta, OptNumber};
use crate::{no_alloc::{opt_header, opt_len_or_delta},
            from_bytes::*};

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

impl<I: Iterator<Item = u8>> TryConsumeBytes<I> for Vec<Opt> {
  type Error = OptParseError;

  fn try_consume_bytes(bytes: &mut I) -> Result<Self, Self::Error> {
    let mut opts = Vec::with_capacity(32);

    loop {
      match Opt::try_consume_bytes(bytes) {
        | Ok(opt) => {
          opts.push(opt);
        },
        | Err(OptParseError::OptionsExhausted) => break Ok(opts),
        | Err(e) => break Err(e),
      }
    }
  }
}
impl<I: Iterator<Item = u8>> TryConsumeBytes<I> for Opt {
  type Error = OptParseError;

  fn try_consume_bytes(bytes: &mut I) -> Result<Self, Self::Error> {
    let opt_header = opt_header(bytes.by_ref())?;

    // NOTE: Delta **MUST** be consumed before Value. see comment on `opt_len_or_delta` for more info
    let delta = OptDelta::try_consume_bytes(&mut core::iter::once(opt_header).chain(bytes.by_ref()))?;
    let len = opt_header & 0b00001111;
    let len = opt_len_or_delta(len, bytes.by_ref(), OptParseError::ValueLengthReservedValue(15))?;
    let value = OptValue::try_consume_n_bytes(len as usize, bytes)?;
    Ok(Opt { delta, value })
  }
}

impl<I: Iterator<Item = u8>> TryConsumeNBytes<I> for OptValue {
  type Error = OptParseError;

  fn try_consume_n_bytes(n: usize, bytes: &mut I) -> Result<Self, Self::Error> {
    let mut data = Vec::<u8>::with_capacity(n as usize);
    data.extend(&mut bytes.take(n));

    if data.len() < n {
      Err(Self::Error::UnexpectedEndOfStream)
    } else {
      Ok(OptValue(data))
    }
  }
}

#[cfg(test)]
mod tests {
  use core::iter::{once, repeat};

  use super::*;
  #[test]
  fn parse_opt_value() {
    let mut val_1byte = once(2);
    let val_1byte = OptValue::try_consume_n_bytes(1, &mut val_1byte).unwrap();
    assert_eq!(val_1byte, OptValue(vec![2]));

    let data13bytes = repeat(1u8).take(13).collect::<Vec<_>>();
    let mut val_13bytes = data13bytes.iter().copied();
    let val_13bytes = OptValue::try_consume_n_bytes(13, &mut val_13bytes).unwrap();
    assert_eq!(val_13bytes, OptValue(data13bytes));

    let data270bytes = repeat(1u8).take(270).collect::<Vec<_>>();
    let mut val_270bytes = data270bytes.iter().copied();
    let val_270bytes = OptValue::try_consume_n_bytes(270, &mut val_270bytes).unwrap();
    assert_eq!(val_270bytes, OptValue(data270bytes));

    let errs = [(1, [].into_iter())];

    errs.into_iter().for_each(|(n, mut bytes)| {
                      let del = OptValue::try_consume_n_bytes(n, &mut bytes);
                      assert_eq!(del, Err(OptParseError::UnexpectedEndOfStream))
                    });
  }

  #[test]
  fn parse_opt() {
    let opt_bytes: [u8; 2] = [0b00000001, 0b00000001];
    let opt = Opt::try_consume_bytes(&mut opt_bytes.into_iter()).unwrap();
    assert_eq!(opt,
               Opt { delta: OptDelta(0),
                     value: OptValue(vec![1]) });

    let opt_bytes: [u8; 5] = [0b00000001, 0b00000001, 0b00010001, 0b00000011, 0b11111111];
    let opt = Vec::<Opt>::try_consume_bytes(&mut opt_bytes.into_iter()).unwrap();
    assert_eq!(opt,
               vec![Opt { delta: OptDelta(0),
                          value: OptValue(vec![1]) },
                    Opt { delta: OptDelta(1),
                          value: OptValue(vec![3]) },]);
  }
}
