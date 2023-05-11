use core::hash::Hash;
use core::iter::FromIterator;
use core::marker::PhantomData;
use core::ops::{Add, Sub};

#[cfg(feature = "alloc")]
use std_alloc::vec::Vec;
use tinyvec::ArrayVec;
use toad_array::{AppendCopy, Array, Indexed};
use toad_cursor::Cursor;
use toad_len::Len;
use toad_macros::rfc_7252_doc;
use toad_map::Map;

use crate::from_bytes::*;

/// Option parsing error
pub mod parse_error;
pub use parse_error::*;

/// Well-known options
pub mod known;
pub use known::*;

use self::no_repeat::{BLOCK1, BLOCK2};

/// An iterator over owned [`Opt`]s
#[derive(Debug, Clone)]
pub struct OptIter<M, I>
  where M: OptionMap
{
  iter: I,
  last_seen_num: OptNumber,
  repeated: Option<(OptNumber, M::OptValues)>,
  __p: PhantomData<M>,
}

/// An iterator over [`OptRef`]s
#[derive(Debug, Clone)]
pub struct OptRefIter<'a, M, I>
  where M: OptionMap
{
  iter: I,
  last_seen_num: OptNumber,
  repeated: Option<(OptNumber, &'a M::OptValues, usize)>,
  __p: PhantomData<M>,
}

impl<M, I> Iterator for OptIter<M, I>
  where I: Iterator<Item = (OptNumber, M::OptValues)>,
        M: OptionMap
{
  type Item = Opt<M::OptValue>;

  fn next(&mut self) -> Option<Self::Item> {
    let (num, values) = Option::take(&mut self.repeated).or_else(|| self.iter.next())?;

    match values.len() {
      | 1 => {
        let OptNumber(delta) = num - self.last_seen_num;
        let delta = OptDelta(delta as u16);
        self.last_seen_num = num;

        Some(Opt { value: values.into_iter().next().unwrap(),
                   delta })
      },
      | _ => {
        let mut values = values.into_iter();
        if let Some(value) = values.next() {
          self.repeated = Some((num, values.collect()));

          let OptNumber(delta) = num - self.last_seen_num;
          let delta = OptDelta(delta as u16);
          self.last_seen_num = num;

          Some(Opt { value, delta })
        } else {
          self.repeated = None;
          self.next()
        }
      },
    }
  }
}

impl<'a, M, I> Iterator for OptRefIter<'a, M, I>
  where I: Iterator<Item = (&'a OptNumber, &'a M::OptValues)>,
        Self: 'a,
        M: 'a + OptionMap
{
  type Item = OptRef<'a, M::OptValue>;

  fn next(&mut self) -> Option<Self::Item> {
    let (num, values, ix) = self.repeated
                                .or_else(|| self.iter.next().map(|(a, b)| (*a, b, 0)))?;

    match values.len() {
      | 1 => {
        let OptNumber(delta) = num - self.last_seen_num;
        let delta = OptDelta(delta as u16);
        self.last_seen_num = num;

        Some(OptRef { value: &values[0],
                      delta })
      },
      | _ => {
        if let Some(value) = values.get(ix) {
          self.repeated = Some((num, values, ix + 1));

          let OptNumber(delta) = num - self.last_seen_num;
          let delta = OptDelta(delta as u16);
          self.last_seen_num = num;

          Some(OptRef { value, delta })
        } else {
          self.repeated = None;
          self.next()
        }
      },
    }
  }
}

/// Generalization of `HashMap<OptNumber, OptValue<Vec<u8>>>`
pub trait OptionMap
  where Self: Map<OptNumber, Self::OptValues>
{
  /// Byte array for option values
  type OptValue: Array<Item = u8> + AppendCopy<u8>;

  /// One or more values for a given number.
  ///
  /// Note that not all options are repeatable.
  type OptValues: Array<Item = OptValue<Self::OptValue>>;

  /// Iterate over the map, yielding raw option structures
  fn opts(self) -> OptIter<Self, Self::IntoIter> {
    OptIter { iter: self.into_iter(),
              last_seen_num: OptNumber(0),
              __p: PhantomData,
              repeated: None }
  }

  /// Iterate over the map, yielding raw option structures
  fn opt_refs(&self) -> OptRefIter<'_, Self, toad_map::Iter<'_, OptNumber, Self::OptValues>> {
    OptRefIter { iter: self.iter(),
                 last_seen_num: OptNumber(0),
                 __p: PhantomData,
                 repeated: None }
  }
}

#[cfg(feature = "alloc")]
impl OptionMap for std_alloc::collections::BTreeMap<OptNumber, Vec<OptValue<Vec<u8>>>> {
  type OptValue = Vec<u8>;
  type OptValues = Vec<OptValue<Vec<u8>>>;
}

type ArrayVecMap<const N: usize, K, V> = ArrayVec<[(K, V); N]>;

impl<const MAX_OPTS: usize, const MAX_INSTANCES: usize, const MAX_BYTES_PER_INSTANCE: usize>
  OptionMap
  for ArrayVecMap<MAX_OPTS,
                  OptNumber,
                  ArrayVec<[OptValue<ArrayVec<[u8; MAX_BYTES_PER_INSTANCE]>>; MAX_INSTANCES]>>
{
  type OptValue = ArrayVec<[u8; MAX_BYTES_PER_INSTANCE]>;
  type OptValues = ArrayVec<[OptValue<Self::OptValue>; MAX_INSTANCES]>;
}

impl<B: AsRef<[u8]>, M: OptionMap> TryConsumeBytes<B> for M {
  type Error = OptParseError;

  fn try_consume_bytes(bytes: &mut Cursor<B>) -> Result<Self, Self::Error> {
    let mut map = Self::default();

    let mut last_inserted = OptNumber(0);

    loop {
      match Opt::try_consume_bytes(bytes) {
        | Ok(opt) => {
          if map.is_full() {
            break Err(Self::Error::TooManyOptions(map.len()));
          }

          let OptDelta(d) = opt.delta;
          let num = last_inserted + OptNumber(d as u32);

          let mut values = M::OptValues::default();
          values.push(opt.value);

          map.insert(num, values).ok();
          last_inserted = num;
        },
        | Err(OptParseError::OptionsExhausted) => break Ok(map),
        | Err(e) => break Err(e),
      }
    }
  }
}

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
/// Low-level representation of a CoAP Option, closely mirroring the byte layout
/// of message options.
///
/// Notably, this doesn't include the Number (key, e.g. "Content-Format" or "Uri-Path").
/// To refer to numbers we use implementors of the [`OptionMap`] trait.
#[derive(Clone, Debug, Default)]
pub struct Opt<C> {
  /// See [`OptDelta`]
  pub delta: OptDelta,
  /// See [`OptValue`]
  pub value: OptValue<C>,
}

impl<C> PartialOrd for Opt<C> where C: Array<Item = u8>
{
  fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
    Some(self.cmp(other))
  }
}

impl<C> PartialEq for Opt<C> where C: Array<Item = u8>
{
  fn eq(&self, other: &Self) -> bool {
    self.delta.eq(&other.delta) && self.value.eq(&other.value)
  }
}

impl<C> Ord for Opt<C> where C: Array<Item = u8>
{
  fn cmp(&self, other: &Self) -> core::cmp::Ordering {
    self.delta
        .cmp(&other.delta)
        .then_with(|| self.value.cmp(&other.value))
  }
}

impl<C> Eq for Opt<C> where C: Array<Item = u8> {}

/// A low-cost copyable [`Opt`] that stores a reference to the value
#[derive(Copy, Clone, Debug)]
#[allow(missing_docs)]
pub struct OptRef<'a, C> {
  pub delta: OptDelta,
  pub value: &'a OptValue<C>,
}

impl<'a, C> PartialOrd for OptRef<'a, C> where C: Array<Item = u8>
{
  fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
    Some(self.cmp(other))
  }
}

impl<'a, C> PartialEq for OptRef<'a, C> where C: Array<Item = u8>
{
  fn eq(&self, other: &Self) -> bool {
    self.delta.eq(&other.delta) && self.value.eq(other.value)
  }
}

impl<'a, C> Ord for OptRef<'a, C> where C: Array<Item = u8>
{
  fn cmp(&self, other: &Self) -> core::cmp::Ordering {
    self.delta
        .cmp(&other.delta)
        .then_with(|| self.value.cmp(other.value))
  }
}

impl<'a, C> Eq for OptRef<'a, C> where C: Array<Item = u8> {}

impl<'a, C: Array<Item = u8>> Len for OptRef<'a, C> {
  const CAPACITY: Option<usize> = None;

  fn len(&self) -> usize {
    let header_size = 1;
    let delta_size = match self.delta.0 {
      | n if n >= 269 => 2,
      | n if n >= 13 => 1,
      | _ => 0,
    };

    let value_len_size = match self.value.0.len() {
      | n if n >= 269 => 2,
      | n if n >= 13 => 1,
      | _ => 0,
    };

    header_size + delta_size + value_len_size + self.value.0.len()
  }

  fn is_full(&self) -> bool {
    false
  }
}

impl<'a, V> From<&'a Opt<V>> for OptRef<'a, V> {
  fn from(o: &'a Opt<V>) -> Self {
    Self { value: &o.value,
           delta: o.delta }
  }
}

impl<C: Array<Item = u8>> Opt<C> {
  /// Given a collection to [`Extend`] and an Opt, add that Opt's bytes to the collection.
  pub fn extend_bytes(self, bytes: &mut impl Extend<u8>) {
    let (del, del_bytes) = crate::to_bytes::opt_len_or_delta(self.delta.0);
    let (len, len_bytes) = crate::to_bytes::opt_len_or_delta(self.value.0.len() as u16);
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
/// # Related
/// - [RFC7252#section-3.1 Option Format](https://datatracker.ietf.org/doc/html/rfc7252#section-3.1)
#[derive(Copy, Clone, Hash, PartialEq, PartialOrd, Eq, Ord, Debug, Default)]
pub struct OptDelta(pub u16);

#[doc = rfc_7252_doc!("5.4.6")]
/// <details><summary><b>RFC7252 Section 12.2 Core CoAP Option Numbers</b></summary>
#[doc = concat!("\n#", rfc_7252_doc!("12.2"))]
/// </details>
#[derive(Copy, Clone, Hash, PartialEq, PartialOrd, Eq, Ord, Debug, Default)]
pub struct OptNumber(pub u32);

impl Add for OptNumber {
  type Output = OptNumber;

  fn add(self, rhs: Self) -> Self::Output {
    Self(self.0 + rhs.0)
  }
}

impl Sub for OptNumber {
  type Output = OptNumber;

  fn sub(self, rhs: Self) -> Self::Output {
    Self(self.0 - rhs.0)
  }
}

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
  /// If this option is [safe to forward](`WhenOptionUnsupportedByProxy::Forward`),
  /// but unknown to a proxy, it should be included in the proxy's
  /// cache key for this message.
  ///
  /// Corresponds to the option being not "NoCacheKey"
  /// in strict CoAP terms
  ResponseChanges,
  /// If this option is [safe to forward](`WhenOptionUnsupportedByProxy::Forward`),
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

  /// Whether this option should be included in the [`Message::cache_key`]
  pub fn include_in_cache_key(&self) -> bool {
    self.when_option_changes() == WhenOptionChanges::ResponseChanges
    && self != &BLOCK1
    && self != &BLOCK2
  }
}

#[doc = rfc_7252_doc!("3.2")]
#[derive(Default, Clone, Debug)]
pub struct OptValue<C>(pub C);

impl<C> PartialOrd for OptValue<C> where C: Array<Item = u8>
{
  fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
    self.0.iter().partial_cmp(other.0.iter())
  }
}

impl<C> PartialEq for OptValue<C> where C: Array<Item = u8>
{
  fn eq(&self, other: &Self) -> bool {
    self.0.iter().eq(other.0.iter())
  }
}

impl<C> Ord for OptValue<C> where C: Array<Item = u8>
{
  fn cmp(&self, other: &Self) -> core::cmp::Ordering {
    self.0.iter().cmp(other.0.iter())
  }
}

impl<C> Eq for OptValue<C> where C: Array<Item = u8> {}

impl<C> Hash for OptValue<C> where C: Array<Item = u8>
{
  fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
    state.write(&self.0)
  }
}

impl<C> OptValue<C> where C: Array<Item = u8>
{
  /// Convert a reference to a OptValue to a byte slice
  pub fn as_bytes(&self) -> &[u8] {
    &self.0
  }
}

impl<C> FromIterator<u8> for OptValue<C> where C: FromIterator<u8>
{
  fn from_iter<T: IntoIterator<Item = u8>>(iter: T) -> Self {
    Self(iter.into_iter().collect::<C>())
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

    if value.len() < len {
      return Err(Self::Error::UnexpectedEndOfStream);
    }

    let value = OptValue(value);

    Ok(Opt { delta, value })
  }
}

#[cfg(test)]
mod tests {
  use std_alloc::collections::BTreeMap;

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
    let opt =
      BTreeMap::<OptNumber, Vec<OptValue<Vec<u8>>>>::try_consume_bytes(&mut opt_bytes).unwrap();
    assert_eq!(opt,
               BTreeMap::from([(OptNumber(0), vec![OptValue(vec![1])]),
                               (OptNumber(1), vec![OptValue(vec![3])])]));
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
