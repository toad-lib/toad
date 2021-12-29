use crate::{GetSize, from_bytes::*, Collection};

/// Low-level representation of a freshly parsed CoAP Option
/// 
/// Both requests and responses may include a list of one or more
/// options. For example, the URI in a request is transported in several
/// options, and metadata that would be carried in an HTTP header in HTTP
/// is supplied as options as well.
/// 
/// ## Option Numbers
/// This struct just stores data parsed directly from the message on the wire,
/// and does not compute or store the Option Number.
/// 
/// To get Option [`OptNumber`]s, you can use the iterator extension [`EnumerateOptNumbers`] on a collection of [`Opt`]s.
/// 
/// # Related
/// - [RFC7252#section-3.1 Option Format](https://datatracker.ietf.org/doc/html/rfc7252#section-3.1)
/// - [RFC7252#section-5.4 Options](https://datatracker.ietf.org/doc/html/rfc7252#section-5.4)
#[derive(Clone, PartialEq, PartialOrd, Debug, Default)]
pub struct Opt<C: Collection<u8>> 
where for<'a> &'a C: IntoIterator<Item = &'a u8>
{
  /// See [`OptDelta`]
  pub delta: OptDelta,
  /// See [`OptValue`]
  pub value: OptValue<C>,
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

/// The Option number identifies which Option is being set (e.g. Content-Format has a Number of 12)
/// 
/// Because Option Numbers are only able to be computed in the context of other options, in order to
/// get Option Numbers you must have a collection of [`Opt`]s.
/// 
/// Then you can use the provided [`EnumerateOptNumbers`] iterator extension to enumerate over options
/// with their numbers.
/// 
/// <details>
/// <summary>Click to see table of Option Numbers defined in the original CoAP RFC</summary>
/// 
/// ```text
/// +--------+------------------+-----------+
/// | Number | Name             | Reference |
/// +--------+------------------+-----------+
/// |      0 | (Reserved)       | [RFC7252] |
/// |      1 | If-Match         | [RFC7252] |
/// |      3 | Uri-Host         | [RFC7252] |
/// |      4 | ETag             | [RFC7252] |
/// |      5 | If-None-Match    | [RFC7252] |
/// |      7 | Uri-Port         | [RFC7252] |
/// |      8 | Location-Path    | [RFC7252] |
/// |     11 | Uri-Path         | [RFC7252] |
/// |     12 | Content-Format   | [RFC7252] |
/// |     14 | Max-Age          | [RFC7252] |
/// |     15 | Uri-Query        | [RFC7252] |
/// |     17 | Accept           | [RFC7252] |
/// |     20 | Location-Query   | [RFC7252] |
/// |     35 | Proxy-Uri        | [RFC7252] |
/// |     39 | Proxy-Scheme     | [RFC7252] |
/// |     60 | Size1            | [RFC7252] |
/// |    128 | (Reserved)       | [RFC7252] |
/// |    132 | (Reserved)       | [RFC7252] |
/// |    136 | (Reserved)       | [RFC7252] |
/// |    140 | (Reserved)       | [RFC7252] |
/// +--------+------------------+-----------+
/// ```
/// </details>
/// 
/// # Related
/// - [RFC7252#section-3.1 Option Format](https://datatracker.ietf.org/doc/html/rfc7252#section-3.1)
/// - [RFC7252#section-5.4.6 Option Numbers](https://datatracker.ietf.org/doc/html/rfc7252#section-5.4.6)
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct OptNumber(pub u32);

/// Option Value
///
/// # Related
/// - [RFC7252#section-3.1 Option Format](https://datatracker.ietf.org/doc/html/rfc7252#section-3.1)
/// - [RFC7252#section-5.4 Options](https://datatracker.ietf.org/doc/html/rfc7252#section-5.4)
#[derive(Default, Clone, PartialEq, PartialOrd, Debug)]
pub struct OptValue<C: Collection<u8>>(pub C)
  where for<'a> &'a C: IntoIterator<Item = &'a u8>
  ;

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

#[doc = include_str!("../docs/parsing/opt_len_or_delta.md")]
pub(crate) fn opt_len_or_delta(head: u8,
                               bytes: impl Iterator<Item = u8>,
                               reserved_err: OptParseError)
                               -> Result<u16, OptParseError> {
  if head == 15 {
    return Err(reserved_err);
  }

  match head {
    | 13 => {
      let n = OptParseError::try_next(bytes)?;
      Ok((n as u16) + 13)
    },
    | 14 => {
      let taken_bytes = bytes.take(2).collect::<tinyvec::ArrayVec<[u8; 2]>>();
      if taken_bytes.is_full() {
        Ok(u16::from_be_bytes(taken_bytes.into_inner()) + 269)
      } else {
        Err(OptParseError::UnexpectedEndOfStream)
      }
    },
    | _ => Ok(head as u16),
  }
}

impl<OptCollection: Collection<Opt<C>>, I: Iterator<Item = u8>, C: 'static + Collection<u8>> TryConsumeBytes<I>
  for OptCollection
where for<'a> &'a OptCollection: IntoIterator<Item = &'a Opt<C>>,
for<'a> &'a C: IntoIterator<Item = &'a u8>

{
  type Error = OptParseError;

  fn try_consume_bytes(bytes: &mut I) -> Result<Self, Self::Error> {
    let mut opts = OptCollection::default();

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

impl<I: Iterator<Item = u8>, C: Collection<u8>> TryConsumeBytes<I> for Opt<C> 
where for<'a> &'a C: IntoIterator<Item = &'a u8>{
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

impl<I: Iterator<Item = u8>, C: Collection<u8>> TryConsumeNBytes<I> for OptValue<C> where for<'a> &'a C: IntoIterator<Item = &'a u8>{
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

impl<C: Collection<u8>, I: Iterator<Item = Opt<C>>> EnumerateOptNumbers<Opt<C>> for I where for<'a> &'a C: IntoIterator<Item = &'a u8>{
  fn enumerate_option_numbers(self) -> EnumerateOptNumbersIter<Opt<C>, Self> {
    EnumerateOptNumbersIter { number: 0, iter: self }
  }
}

impl<'a, C: Collection<u8>, I: Iterator<Item = &'a Opt<C>>> EnumerateOptNumbers<&'a Opt<C>> for I where for<'b> &'b C: IntoIterator<Item = &'b u8>{
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

impl<C: Collection<u8>, I: Iterator<Item = Opt<C>>> Iterator for EnumerateOptNumbersIter<Opt<C>, I> where for<'a> &'a C: IntoIterator<Item = &'a u8>{
  type Item = (OptNumber, Opt<C>);

  fn next(&mut self) -> Option<Self::Item> {
    self.iter.next().map(|opt| {
      self.number += opt.delta.0 as u32;
      (OptNumber(self.number), opt)
    })
  }
}

impl<'a, C: Collection<u8>, I: Iterator<Item = &'a Opt<C>>> Iterator for EnumerateOptNumbersIter<&'a Opt<C>, I> where for<'b> &'b C: IntoIterator<Item = &'b u8>{
  type Item = (OptNumber, &'a Opt<C>);

  fn next(&mut self) -> Option<Self::Item> {
    self.iter.next().map(|opt| {
      self.number += opt.delta.0 as u32;
      (OptNumber(self.number), opt)
    })
  }
}

#[cfg(never)]
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
