use kwap_common::{Array, GetSize};
use kwap_macros::rfc_7252_doc;

use crate::from_bytes::*;

#[doc = rfc_7252_doc!("5.4")]
/// <details><summary><b>RFC7252 Section 3.1 Option binary format</b></summary>
#[doc = concat!("\n#", rfc_7252_doc!("3.1"))]
/// </details>
///
/// # `Opt` struct
/// Low-level representation of a freshly parsed CoAP Option
///
/// ## Option Numbers
/// This struct just stores data parsed directly from the message on the wire,
/// and does not compute or store the Option Number.
///
/// To get [`OptNumber`]s, you can use the iterator extension [`EnumerateOptNumbers`] on a collection of [`Opt`]s.
#[derive(Clone, PartialEq, PartialOrd, Debug, Default)]
pub struct Opt<C: Array<Item = u8>> {
  /// See [`OptDelta`]
  pub delta: OptDelta,
  /// See [`OptValue`]
  pub value: OptValue<C>,
}

impl<C: Array<Item = u8>> GetSize for Opt<C> {
  fn get_size(&self) -> usize {
    let header_size = 1;
    let delta_size = match self.delta.0 {
      | n if n >= 269 => 2,
      | n if n >= 13 => 1,
      | _ => 0,
    };

    let value_len_size = match self.value.0.get_size() {
      | n if n >= 269 => 2,
      | n if n >= 13 => 1,
      | _ => 0,
    };

    header_size + delta_size + value_len_size + self.value.0.get_size()
  }

  fn max_size(&self) -> Option<usize> {
    None
  }
}

impl<C: Array<Item = u8>> Opt<C> {
  /// Given a collection to [`Extend`] and an Opt, add that Opt's bytes to the collection.
  pub fn extend_bytes(self, bytes: &mut impl Extend<u8>) {
    let (del, del_bytes) = crate::to_bytes::opt_len_or_delta(self.delta.0);
    let (len, len_bytes) = crate::to_bytes::opt_len_or_delta(self.value.0.get_size() as u16);
    let del = del << 4;

    let header = del | len;

    bytes.extend(Some(header));

    if let Some(bs) = del_bytes {
      bytes.extend(bs);
    }

    if let Some(bs) = len_bytes {
      bytes.extend(bs);
    }

    bytes.extend(self.value.0);
  }
}

/// The "Option Delta" is the difference between this Option's Number
/// and the previous Option's number.
///
/// This is just used to compute the Option Number, identifying which
/// Option is being set (e.g. Content-Format has a Number of 12)
///
/// To use this to get Option Numbers, see [`EnumerateOptNumbers`].
///
/// # Related
/// - [RFC7252#section-3.1 Option Format](https://datatracker.ietf.org/doc/html/rfc7252#section-3.1)
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Default)]
pub struct OptDelta(pub u16);

#[doc = rfc_7252_doc!("5.4.6")]
/// <details><summary><b>RFC7252 Section 12.2 Core CoAP Option Numbers</b></summary>
#[doc = concat!("\n#", rfc_7252_doc!("12.2"))]
/// </details>
///
/// # `OptNumber` struct
/// Because Option Numbers are only able to be computed in the context of other options, in order to
/// get Option Numbers you must have a collection of [`Opt`]s, and use the provided [`EnumerateOptNumbers`].
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Default)]
pub struct OptNumber(pub u32);

#[doc = rfc_7252_doc!("3.2")]
#[derive(Default, Clone, PartialEq, PartialOrd, Debug)]
pub struct OptValue<C: Array<Item = u8>>(pub C);

/// Peek at the first byte of a byte iterable and interpret as an Option header.
///
/// This converts the iterator into a Peekable and looks at bytes0.
/// Checks if byte 0 is a Payload marker, indicating all options have been read.
pub(crate) fn opt_header<I: Iterator<Item = u8>>(mut bytes: I) -> Result<u8, OptParseError> {
  let opt_header = bytes.next();

  if let Some(0b11111111) | None = opt_header {
    // This isn't an option, it's the payload!
    return Err(OptParseError::OptionsExhausted);
  }

  Ok(opt_header.unwrap())
}

impl<C: Array<Item = u8>, OptArray: Array<Item = Opt<C>>, I: Iterator<Item = u8>> TryConsumeBytes<I> for OptArray {
  type Error = OptParseError;

  fn try_consume_bytes(bytes: &mut I) -> Result<Self, Self::Error> {
    let mut opts = OptArray::default();

    loop {
      match Opt::try_consume_bytes(bytes) {
        | Ok(opt) => {
          if !opts.is_full() {
            opts.extend(Some(opt));
          } else {
            break Err(Self::Error::TooManyOptions(opts.get_size()));
          }
        },
        | Err(OptParseError::OptionsExhausted) => break Ok(opts),
        | Err(e) => break Err(e),
      }
    }
  }
}

impl<I: Iterator<Item = u8>, C: Array<Item = u8>> TryConsumeBytes<I> for Opt<C> {
  type Error = OptParseError;

  fn try_consume_bytes(bytes: &mut I) -> Result<Self, Self::Error> {
    let opt_header = opt_header(bytes.by_ref())?;

    // NOTE: Delta **MUST** be consumed before Value. see comment on `opt_len_or_delta` for more info
    let delta = OptDelta::try_consume_bytes(&mut core::iter::once(opt_header).chain(bytes.by_ref()))?;
    let len = opt_header & 0b00001111;
    let len = parse_opt_len_or_delta(len, bytes.by_ref(), OptParseError::ValueLengthReservedValue(15))?;
    let value = OptValue::try_consume_n_bytes(len as usize, bytes)?;
    Ok(Opt { delta, value })
  }
}

impl<I: Iterator<Item = u8>, C: Array<Item = u8>> TryConsumeNBytes<I> for OptValue<C> {
  type Error = OptParseError;

  fn try_consume_n_bytes(n: usize, bytes: &mut I) -> Result<Self, Self::Error> {
    let mut data = C::reserve(n);
    data.extend(&mut bytes.take(n));

    if data.get_size() < n {
      Err(Self::Error::UnexpectedEndOfStream)
    } else {
      Ok(OptValue(data))
    }
  }
}

/// Creates an iterator which gives the current opt's number as well as the option.
///
/// The iterator returned yields pairs `(i, val)`, where `i` is the [`OptNumber`] and `val` is the Opt returned by the iterator.
pub trait EnumerateOptNumbers<T>
  where Self: Sized + Iterator<Item = T>
{
  /// Creates an iterator which gives the current Opt along with its Number.
  ///
  /// ```
  /// use kwap_msg::*;
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

impl<C: Array<Item = u8>, I: Iterator<Item = Opt<C>>> EnumerateOptNumbers<Opt<C>> for I {
  fn enumerate_option_numbers(self) -> EnumerateOptNumbersIter<Opt<C>, Self> {
    EnumerateOptNumbersIter { number: 0, iter: self }
  }
}

impl<'a, C: Array<Item = u8>, I: Iterator<Item = &'a Opt<C>>> EnumerateOptNumbers<&'a Opt<C>> for I {
  fn enumerate_option_numbers(self) -> EnumerateOptNumbersIter<&'a Opt<C>, Self> {
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
pub struct EnumerateOptNumbersIter<T, I: Iterator<Item = T>> {
  number: u32,
  iter: I,
}

impl<C: Array<Item = u8>, I: Iterator<Item = Opt<C>>> Iterator for EnumerateOptNumbersIter<Opt<C>, I> {
  type Item = (OptNumber, Opt<C>);

  fn next(&mut self) -> Option<Self::Item> {
    self.iter.next().map(|opt| {
                      self.number += opt.delta.0 as u32;
                      (OptNumber(self.number), opt)
                    })
  }
}

impl<'a, C: Array<Item = u8>, I: Iterator<Item = &'a Opt<C>>> Iterator for EnumerateOptNumbersIter<&'a Opt<C>, I> {
  type Item = (OptNumber, &'a Opt<C>);

  fn next(&mut self) -> Option<Self::Item> {
    self.iter.next().map(|opt| {
                      self.number += opt.delta.0 as u32;
                      (OptNumber(self.number), opt)
                    })
  }
}

#[cfg(test)]
mod tests {
  use core::iter::{once, repeat};

  use super::*;

  #[test]
  fn parse_opt_delta() {
    let mut del_4bit = [0b00010000u8].into_iter();
    let del_4bit = OptDelta::try_consume_bytes(&mut del_4bit).unwrap();
    assert_eq!(del_4bit, OptDelta(1));

    let mut del_1byte = [0b11010000u8, 0b00000000].into_iter();
    let del_1byte = OptDelta::try_consume_bytes(&mut del_1byte).unwrap();
    assert_eq!(del_1byte, OptDelta(13));

    let mut del_2bytes = [[0b11100000u8].as_ref(), u16::to_be_bytes(12076).as_ref()].concat()
                                                                                    .into_iter();
    let del_2bytes = OptDelta::try_consume_bytes(&mut del_2bytes).unwrap();
    assert_eq!(del_2bytes, OptDelta(12345));

    let errs = [[0b11010000u8].as_ref().iter(),             // delta is 13 but no byte following
                [0b11100000u8, 0b00000001].as_ref().iter(), // delta is 14 but only 1 byte following
                [].as_ref().iter()];

    errs.into_iter().for_each(|iter| {
                      let del = OptDelta::try_consume_bytes(&mut iter.copied());
                      assert_eq!(del, Err(OptParseError::UnexpectedEndOfStream))
                    });
  }

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
                      let del = OptValue::<Vec<_>>::try_consume_n_bytes(n, &mut bytes);
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
    let opt = Vec::<Opt<Vec<_>>>::try_consume_bytes(&mut opt_bytes.into_iter()).unwrap();
    assert_eq!(opt,
               vec![Opt { delta: OptDelta(0),
                          value: OptValue(vec![1]) },
                    Opt { delta: OptDelta(1),
                          value: OptValue(vec![3]) },]);
  }
}
