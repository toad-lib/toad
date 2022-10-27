use toad_common::{AppendCopy, Array, Cursor, GetSize};
use toad_macros::rfc_7252_doc;

use crate::from_bytes::*;

pub mod parse_error;
pub use parse_error::*;

pub(crate) fn parse_opt_len_or_delta<A: AsRef<[u8]>>(head: u8,
                                                     bytes: &mut Cursor<A>,
                                                     reserved_err: OptParseError)
                                                     -> Result<u16, OptParseError> {
  match head {
    | 13 => {
      let n = bytes.next().ok_or_else(OptParseError::eof)?;
      Ok((n as u16) + 13)
    },
    | 14 => match bytes.take_exact(2) {
      | Some(&[a, b]) => Ok(u16::from_be_bytes([a, b]) + 269),
      | _ => Err(OptParseError::eof()),
    },
    | 15 => Err(reserved_err),
    | _ => Ok(head as u16),
  }
}

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
pub struct Opt<C> {
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

  fn is_full(&self) -> bool {
    false
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
#[derive(Copy, Clone, Hash, PartialEq, PartialOrd, Debug, Default)]
pub struct OptDelta(pub u16);

#[doc = rfc_7252_doc!("5.4.6")]
/// <details><summary><b>RFC7252 Section 12.2 Core CoAP Option Numbers</b></summary>
#[doc = concat!("\n#", rfc_7252_doc!("12.2"))]
/// </details>
///
/// # `OptNumber` struct
/// Because Option Numbers are only able to be computed in the context of other options, in order to
/// get Option Numbers you must have a collection of [`Opt`]s, and use the provided [`EnumerateOptNumbers`].
#[derive(Copy, Clone, Hash, PartialEq, PartialOrd, Debug, Default)]
pub struct OptNumber(pub u32);

#[doc = rfc_7252_doc!("5.4.1")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum OptionMustBeProcessed {
  /// This option must be processed,
  /// and a response that ignores it
  /// will be rejected.
  ///
  /// Corresponds to the option being "critical"
  /// in strict CoAP terms
  Yes,
  /// This option does not _need_ to
  /// be processed,
  /// and a response that ignores it
  /// will be processed anyway.
  ///
  /// Corresponds to the option being "elective"
  /// in strict CoAP terms
  No,
}

#[doc = rfc_7252_doc!("5.4.2")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum WhenOptionUnsupportedByProxy {
  /// This option /must be/ processed & understood by proxies
  /// and may not be forwarded blindly to their destination.
  ///
  /// Corresponds to the option being "UnSafe" to forward
  /// in strict CoAP terms
  Error,
  /// This option may not be processed & understood by proxies
  /// and may be forwarded blindly to their destination.
  ///
  /// Corresponds to the option being "SafeToForward"
  /// in strict CoAP terms
  Forward,
}

#[doc = rfc_7252_doc!("5.4.2")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum WhenOptionChanges {
  /// If this option is [safe to forward](`ProxySafe::ForwardWhenUnknown`),
  /// but unknown to a proxy, it should be included in the proxy's
  /// cache key for this message.
  ///
  /// Corresponds to the option being not "NoCacheKey"
  /// in strict CoAP terms
  ResponseChanges,
  /// If this option is [safe to forward](`ProxySafe::ForwardWhenUnknown`),
  /// but unknown to a proxy, it should not be included in the proxy's
  /// cache key for this message, and different values for this option
  /// should yield the cached response.
  ///
  /// Corresponds to the option being "NoCacheKey"
  /// in strict CoAP terms
  ResponseDoesNotChange,
}

impl OptNumber {
  /// Whether or not this option may be ignored by a server
  pub fn must_be_processed(&self) -> OptionMustBeProcessed {
    #[allow(clippy::wildcard_in_or_patterns)] // will only ever be 0 or 1
    match self.0 & 0b1 {
      | 1 => OptionMustBeProcessed::Yes,
      | 0 | _ => OptionMustBeProcessed::No,
    }
  }

  /// Whether or not this option may be forwarded blindly by
  /// a proxy that does not support processing it
  pub fn when_unsupported_by_proxy(&self) -> WhenOptionUnsupportedByProxy {
    #[allow(clippy::wildcard_in_or_patterns)] // will only ever be 0 or 1
    match (self.0 & 0b10) >> 1 {
      | 1 => WhenOptionUnsupportedByProxy::Error,
      | 0 | _ => WhenOptionUnsupportedByProxy::Forward,
    }
  }

  /// Whether or not different values for this option should
  /// yield proxies' cached response
  ///
  /// _(when the proxy does not support processing it and
  /// the option is safe to forward)_
  pub fn when_option_changes(&self) -> WhenOptionChanges {
    match (self.0 & 0b11100) >> 2 {
      | 0b111 => WhenOptionChanges::ResponseDoesNotChange,
      | _ => WhenOptionChanges::ResponseChanges,
    }
  }
}

#[doc = rfc_7252_doc!("3.2")]
#[derive(Default, Clone, Hash, PartialEq, PartialOrd, Debug)]
pub struct OptValue<C>(pub C);

impl<V: Array<Item = u8> + AppendCopy<u8>, T: Array<Item = Opt<V>>, Bytes: AsRef<[u8]>>
  TryConsumeBytes<Bytes> for T
{
  type Error = OptParseError;

  fn try_consume_bytes(bytes: &mut Cursor<Bytes>) -> Result<Self, Self::Error> {
    let mut opts = T::default();

    loop {
      match Opt::try_consume_bytes(bytes) {
        | Ok(opt) => {
          if opts.is_full() {
            break Err(Self::Error::TooManyOptions(opts.get_size()));
          }

          opts.push(opt);
        },
        | Err(OptParseError::OptionsExhausted) => break Ok(opts),
        | Err(e) => break Err(e),
      }
    }
  }
}

impl<Bytes: AsRef<[u8]>, V: Array<Item = u8> + AppendCopy<u8>> TryConsumeBytes<Bytes> for Opt<V> {
  type Error = OptParseError;

  fn try_consume_bytes(bytes: &mut Cursor<Bytes>) -> Result<Self, Self::Error> {
    let byte1 = bytes.next()
                     .ok_or(OptParseError::OptionsExhausted)
                     .and_then(|b| {
                       if b == 0b11111111 {
                         Err(OptParseError::OptionsExhausted)
                       } else {
                         Ok(b)
                       }
                     })?;

    // NOTE: Delta **MUST** be consumed before Value. see comment on `opt_len_or_delta` for more info
    let delta = parse_opt_len_or_delta(byte1 >> 4,
                                       bytes,
                                       OptParseError::OptionDeltaReservedValue(15))?;
    let delta = OptDelta(delta);

    let len = parse_opt_len_or_delta(byte1 & 0b00001111,
                                     bytes,
                                     OptParseError::ValueLengthReservedValue(15))?
              as usize;

    let mut value = V::reserve(len);
    value.append_copy(bytes.take(len));

    if value.get_size() < len {
      return Err(Self::Error::UnexpectedEndOfStream);
    }

    let value = OptValue(value);

    Ok(Opt { delta, value })
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
  /// use toad_msg::*;
  ///
  /// let opt_a = Opt { delta: OptDelta(12),
  ///                   value: OptValue(Vec::new()) };
  /// let opt_b = Opt { delta: OptDelta(2),
  ///                   value: OptValue(Vec::new()) };
  /// let opts = vec![opt_a.clone(), opt_b.clone()];
  ///
  /// let opt_ns = opts.into_iter()
  ///                  .enumerate_option_numbers()
  ///                  .collect::<Vec<_>>();
  ///
  /// assert_eq!(opt_ns, vec![(OptNumber(12), opt_a), (OptNumber(14), opt_b)])
  /// ```
  fn enumerate_option_numbers(self) -> EnumerateOptNumbersIter<T, Self>;
}

impl<C: Array<Item = u8>, I: Iterator<Item = Opt<C>>> EnumerateOptNumbers<Opt<C>> for I {
  fn enumerate_option_numbers(self) -> EnumerateOptNumbersIter<Opt<C>, Self> {
    EnumerateOptNumbersIter { number: 0,
                              iter: self }
  }
}

impl<'a, C: Array<Item = u8>, I: Iterator<Item = &'a Opt<C>>> EnumerateOptNumbers<&'a Opt<C>>
  for I
{
  fn enumerate_option_numbers(self) -> EnumerateOptNumbersIter<&'a Opt<C>, Self> {
    EnumerateOptNumbersIter { number: 0,
                              iter: self }
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

impl<C: Array<Item = u8>, I: Iterator<Item = Opt<C>>> Iterator
  for EnumerateOptNumbersIter<Opt<C>, I>
{
  type Item = (OptNumber, Opt<C>);

  fn next(&mut self) -> Option<Self::Item> {
    self.iter.next().map(|opt| {
                      self.number += opt.delta.0 as u32;
                      (OptNumber(self.number), opt)
                    })
  }
}

impl<'a, C: Array<Item = u8>, I: Iterator<Item = &'a Opt<C>>> Iterator
  for EnumerateOptNumbersIter<&'a Opt<C>, I>
{
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
  use super::*;

  #[test]
  fn parse_opt() {
    let mut opt_bytes = Cursor::new([0b00010001, 0b00000001]);
    let opt = Opt::try_consume_bytes(&mut opt_bytes).unwrap();
    assert_eq!(opt,
               Opt { delta: OptDelta(1),
                     value: OptValue(vec![1]) });

    let mut opt_bytes = Cursor::new([0b11010001, 0b00000001, 0b00000001]);
    let opt = Opt::try_consume_bytes(&mut opt_bytes).unwrap();
    assert_eq!(opt,
               Opt { delta: OptDelta(14),
                     value: OptValue(vec![1]) });

    let mut opt_bytes = Cursor::new([0b11100001, 0b00000000, 0b00000001, 0b00000001]);
    let opt = Opt::try_consume_bytes(&mut opt_bytes).unwrap();
    assert_eq!(opt,
               Opt { delta: OptDelta(270),
                     value: OptValue(vec![1]) });

    let mut opt_bytes = Cursor::new([0b00000001, 0b00000001]);
    let opt = Opt::try_consume_bytes(&mut opt_bytes).unwrap();
    assert_eq!(opt,
               Opt { delta: OptDelta(0),
                     value: OptValue(vec![1]) });

    let mut opt_bytes = Cursor::new([0b00000001, 0b00000001, 0b00010001, 0b00000011, 0b11111111]);
    let opt = Vec::<Opt<Vec<_>>>::try_consume_bytes(&mut opt_bytes).unwrap();
    assert_eq!(opt,
               vec![Opt { delta: OptDelta(0),
                          value: OptValue(vec![1]) },
                    Opt { delta: OptDelta(1),
                          value: OptValue(vec![3]) },]);
  }

  #[test]
  fn opt_number_qualities() {
    // critical, safe-to-fwd, cache-key
    let if_match = OptNumber(1);

    // critical, unsafe-to-fwd, cache-key
    let uri_host = OptNumber(3);

    // elective, safe-to-fwd, cache-key
    let etag = OptNumber(4);

    // elective, safe-to-fwd, no-cache-key
    let size1 = OptNumber(60);

    [&if_match, &uri_host].into_iter()
                          .for_each(|num| {
                            assert_eq!(num.must_be_processed(), OptionMustBeProcessed::Yes);
                          });

    [&etag, &size1].into_iter().for_each(|num| {
                                 assert_eq!(num.must_be_processed(), OptionMustBeProcessed::No);
                               });

    [&if_match, &etag, &size1].into_iter().for_each(|num| {
                                            assert_eq!(num.when_unsupported_by_proxy(),
                                                       WhenOptionUnsupportedByProxy::Forward);
                                          });

    [&uri_host].into_iter().for_each(|num| {
                             assert_eq!(num.when_unsupported_by_proxy(),
                                        WhenOptionUnsupportedByProxy::Error);
                           });

    [&if_match, &uri_host, &etag].into_iter().for_each(|num| {
                                               assert_eq!(num.when_option_changes(),
                                                          WhenOptionChanges::ResponseChanges);
                                             });

    [&size1].into_iter().for_each(|num| {
                          assert_eq!(num.when_option_changes(),
                                     WhenOptionChanges::ResponseDoesNotChange);
                        });
  }
}
