use arrayvec::ArrayVec;

use crate::parsing::*;

#[doc = include_str!("../../docs/no_alloc/opt/Opt.md")]
#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub struct Opt<const CAP: usize> {
  /// See [`OptDelta`]
  pub delta: OptDelta,
  /// See [`OptValue`]
  pub value: OptValue<CAP>,
}

impl<const CAP: usize> GetOptDelta for Opt<CAP> {
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
pub struct OptValue<const CAP: usize>(pub ArrayVec<u8, CAP>);

#[doc = include_str!("../../docs/no_alloc/opt/OptDelta.md")]
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct OptDelta(pub u16);

#[doc = include_str!("../../docs/no_alloc/opt/OptNumber.md")]
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct OptNumber(pub u32);

impl<T: IntoIterator<Item = u8>> TryConsumeBytes<T> for OptDelta {
  type Error = OptParseError;

  fn try_consume_bytes(bytes: T) -> Result<Self, Self::Error> {
    let mut bytes = bytes.into_iter();
    let first_byte = Self::Error::try_next(&mut bytes)?;
    let delta = first_byte >> 4;
    let delta = opt_len_or_delta(delta, &mut bytes, OptParseError::OptionDeltaReservedValue(15))?;

    Ok(OptDelta(delta))
  }
}

impl<T: IntoIterator<Item = u8>, const N_OPTS: usize, const OPT_CAP: usize> TryConsumeBytes<T>
  for ArrayVec<Opt<OPT_CAP>, N_OPTS>
{
  type Error = OptParseError;

  fn try_consume_bytes(bytes: T) -> Result<Self, Self::Error> {
    let mut bytes = bytes.into_iter();
    let mut opts = ArrayVec::<_, N_OPTS>::new();

    loop {
      match Opt::<OPT_CAP>::try_consume_bytes(bytes.by_ref()) {
        | Ok(opt) => {
          opts.try_push(opt).map_err(|_| OptParseError::TooManyOptions(N_OPTS))?;
        },
        | Err(OptParseError::OptionsExhausted) => break Ok(opts),
        | Err(e) => break Err(e),
      }
    }
  }
}

impl<T: IntoIterator<Item = u8>, const OPT_CAP: usize> TryConsumeBytes<T> for Opt<OPT_CAP> {
  type Error = OptParseError;

  fn try_consume_bytes(bytes: T) -> Result<Self, Self::Error> {
    let (opt_header, mut bytes) = opt_header(bytes)?;

    // NOTE: Delta **MUST** be consumed before Value. see comment on `opt_len_or_delta` for more info
    let delta = OptDelta::try_consume_bytes(&mut bytes)?;
    let value = OptValue::<OPT_CAP>::try_consume_bytes(&mut [opt_header].into_iter().chain(bytes))?;
    Ok(Opt { delta, value })
  }
}

/// Peek at the first byte of a byte iterable and interpret as an Option header.
///
/// This converts the iterator into a Peekable and looks at bytes0.
/// Checks if byte 0 is a Payload marker, indicating all options have been read.
pub(crate) fn opt_header<I: IntoIterator<Item = u8>>(bytes: I)
                                                     -> Result<(u8, impl Iterator<Item = u8>), OptParseError> {
  let mut bytes = bytes.into_iter().peekable();
  let opt_header = bytes.peek().copied().ok_or(OptParseError::UnexpectedEndOfStream)?;

  if let 0b11111111 = opt_header {
    // This isn't an option, it's the payload!
    Err(OptParseError::OptionsExhausted)?
  }

  Ok((opt_header, bytes))
}

#[doc = include_str!("../../docs/parsing/opt_len_or_delta.md")]
pub(crate) fn opt_len_or_delta(head: u8,
                               bytes: &mut impl Iterator<Item = u8>,
                               reserved_err: OptParseError)
                               -> Result<u16, OptParseError> {
  if head == 15 {
    Err(reserved_err)?
  }

  match head {
    | 13 => {
      let n = OptParseError::try_next(bytes)?;
      Ok((n as u16) + 13)
    },
    | 14 => bytes.take(2)
                 .collect::<arrayvec::ArrayVec<_, 2>>()
                 .into_inner()
                 .map(|array| u16::from_be_bytes(array) + 269)
                 .map_err(|_| OptParseError::UnexpectedEndOfStream),
    | _ => Ok(head as u16),
  }
}

impl<T: IntoIterator<Item = u8>, const OPT_CAP: usize> TryConsumeBytes<T> for OptValue<OPT_CAP> {
  type Error = OptParseError;

  fn try_consume_bytes(bytes: T) -> Result<Self, Self::Error> {
    let mut bytes = bytes.into_iter();
    let first_byte = Self::Error::try_next(&mut bytes)?;
    let len = first_byte & 0b00001111;
    let len = opt_len_or_delta(len, &mut bytes, OptParseError::ValueLengthReservedValue(15))? as usize;

    if len > OPT_CAP {
      Err(OptParseError::OptionValueTooLong { capacity: OPT_CAP,
                                              actual: len })?
    }

    let data: ArrayVec<u8, OPT_CAP> = bytes.take(len).collect();
    if data.len() < len {
      Err(OptParseError::UnexpectedEndOfStream)
    } else {
      Ok(OptValue(data))
    }
  }
}

/// Trait for getting the delta from either heap or heapless Opts
pub trait GetOptDelta {
  /// ```
  /// use kwap_msg::{alloc::{GetOptDelta, Opt, OptDelta, OptValue},
  ///                no_alloc};
  ///
  /// let heaped = Opt { delta: OptDelta(1),
  ///                    value: OptValue(vec![]) };
  /// let stackd = no_alloc::Opt::<128> { delta: OptDelta(1),
  ///                                     value: no_alloc::OptValue(arrayvec::ArrayVec::new()) };
  ///
  /// assert_eq!(heaped.get_delta(), stackd.get_delta());
  /// ```
  fn get_delta(&self) -> OptDelta;
}

/// Creates an iterator which gives the current opt's number as well as the option.
///
/// The iterator returned yields pairs `(i, val)`, where `i` is the [`OptNumber`] and `val` is the Opt returned by the iterator.
pub trait EnumerateOptNumbers<T: GetOptDelta>: Iterator<Item = T>
  where Self: Sized
{
  /// Creates an iterator which gives the current Opt along with its Number.
  ///
  /// ```
  /// use kwap_msg::alloc::*;
  ///
  /// let opt_a = Opt { delta: OptDelta(12),
  ///                   value: OptValue(Vec::new()) };
  /// let opt_b = Opt { delta: OptDelta(2),
  ///                   value: OptValue(Vec::new()) };
  /// let opts = vec![opt_a.clone(), opt_b.clone()];
  ///
  /// let opt_ns = opts.into_iter().enumerate_option_numbers().collect::<Vec<_>>();
  ///
  /// assert_eq!(opt_ns, vec![(OptNumber(12), opt_a), (OptNumber(14), opt_b)])
  /// ```
  fn enumerate_option_numbers(self) -> EnumerateOptNumbersIter<T, Self>;
}

impl<T: GetOptDelta, I: Iterator<Item = T>> EnumerateOptNumbers<T> for I {
  fn enumerate_option_numbers(self) -> EnumerateOptNumbersIter<T, Self> {
    EnumerateOptNumbersIter { number: 0, iter: self }
  }
}

/// Iterator yielded by [`EnumerateOptNumbers`], wrapping an Iterator
/// over [`Opt`]s.
///
/// Invoking [`Iterator::next`] on this struct will advance the
/// inner iterator, and add the delta of the new opt to its running sum of deltas.
///
/// This running sum is the Number of the newly iterated Opt.
#[derive(Clone, Debug)]
pub struct EnumerateOptNumbersIter<T: GetOptDelta, I: Iterator<Item = T>> {
  number: u32,
  iter: I,
}

/// impl Iterator for EnumerateOptNumbersIter
impl<T: GetOptDelta, I: Iterator<Item = T>> Iterator for EnumerateOptNumbersIter<T, I> {
  type Item = (OptNumber, T);

  fn next(&mut self) -> Option<Self::Item> {
    if let Some(next) = self.iter.next() {
      self.number += u32::from(next.get_delta().0);
      Some((OptNumber(self.number), next))
    } else {
      None
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_opt_delta() {
    let del_4bit = [0b00010000u8];
    let del_4bit = OptDelta::try_consume_bytes(del_4bit).unwrap();
    assert_eq!(del_4bit, OptDelta(1));

    let del_1byte = [0b11010000u8, 0b00000000];
    let del_1byte = OptDelta::try_consume_bytes(del_1byte).unwrap();
    assert_eq!(del_1byte, OptDelta(13));

    let del_2bytes = [[0b11100000u8].as_ref(), u16::to_be_bytes(12076).as_ref()].concat();
    let del_2bytes = OptDelta::try_consume_bytes(del_2bytes).unwrap();
    assert_eq!(del_2bytes, OptDelta(12345));

    let errs = [[0b11010000u8].as_ref(),           // delta is 13 but no byte following
                [0b11100000, 0b00000001].as_ref(), // delta is 14 but only 1 byte following
                [].as_ref()];

    errs.into_iter().for_each(|iter| {
                      let del = OptDelta::try_consume_bytes(iter.to_vec());
                      assert_eq!(del, Err(OptParseError::UnexpectedEndOfStream))
                    });
  }
  #[test]
  fn parse_opt_value() {
    let val_1byte: [u8; 2] = [0b00000001, 2];
    let val_1byte: OptValue<1> = OptValue::try_consume_bytes(val_1byte).unwrap();
    assert_eq!(val_1byte, OptValue([2].into_iter().collect()));

    let data13bytes = core::iter::repeat(1u8).take(13).collect::<Vec<_>>();
    let val_13bytes = [[0b00001101u8, 0b00000000].as_ref(), &data13bytes].concat();
    let val_13bytes: OptValue<13> = OptValue::try_consume_bytes(val_13bytes).unwrap();
    assert_eq!(val_13bytes, OptValue(data13bytes.into_iter().collect()));

    let data270bytes = core::iter::repeat(1u8).take(270).collect::<Vec<_>>();
    let val_270bytes = [[0b00001110u8, 0b00000000, 0b00000001].as_ref(), &data270bytes].concat();
    let val_270bytes: OptValue<270> = OptValue::try_consume_bytes(val_270bytes).unwrap();
    assert_eq!(val_270bytes, OptValue::<270>(data270bytes.into_iter().collect()));

    let errs = [[1].as_ref(),          // len is 1 but no data following
                [13].as_ref(),         // len value is 13, but no data following
                [14, 1].as_ref(),      // len value is 14 but only 1 byte following
                [3, 1, 1, 1].as_ref(), // len is > capacity
                [].as_ref()];

    errs.into_iter().for_each(|iter| {
                      let del = OptValue::<2>::try_consume_bytes(iter.into_iter().copied());
                      assert!(matches!(del,
                                       Err(OptParseError::UnexpectedEndOfStream
                                           | OptParseError::OptionValueTooLong { .. })))
                    });
  }

  #[test]
  fn parse_opt() {
    let opt_bytes: [u8; 2] = [0b00000001, 0b00000001];
    let opt = Opt::<1>::try_consume_bytes(opt_bytes).unwrap();
    assert_eq!(opt,
               Opt { delta: OptDelta(0),
                     value: OptValue::<1>(vec![1].into_iter().collect()) });

    let opt_bytes: [u8; 5] = [0b00000001, 0b00000001, 0b00010001, 0b00000011, 0b11111111];
    let opt = ArrayVec::<Opt<1>, 2>::try_consume_bytes(opt_bytes).unwrap();
    assert_eq!(opt,
               vec![Opt { delta: OptDelta(0),
                          value: OptValue::<1>(vec![1].into_iter().collect()) },
                    Opt { delta: OptDelta(1),
                          value: OptValue::<1>(vec![3].into_iter().collect()) },].into_iter()
                                                                                 .collect::<ArrayVec<_, 2>>());
  }
}
