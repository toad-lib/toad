use crate::bytes::TryConsumeBytes;

#[cfg(feature = "alloc")]
use ::alloc::vec::Vec;
use arrayvec::ArrayVec;

// Module structure:
// opt            - shared traits and structs; OptDelta, OptNumber, EnumerateOptNumbers, GetOptDelta
// opt::opt_alloc - Opt implementation that uses Vec as the backing structure
// opt::opt_fixed - Opt impl, generic over a fixed usize capacity, uses ArrayVec
// opt::bytes     - Some parsing helper functions shared across implementations of TryConsumeBytes

/// Dynamically growable `Opt`. For fixed-capacity non-`alloc` builds, see [`opt_fixed::Opt`].
#[cfg(feature = "alloc")]
#[cfg_attr(any(feature = "docs", docsrs), doc(cfg(feature = "alloc")))]
pub mod opt_alloc;

/// Fixed-capacity `Opt` and `Value`. For the dynamically growable version available with crate feature `alloc`, see [`opt_alloc::Opt`].
pub mod opt_fixed;

#[doc(hidden)]
pub mod bytes;

#[doc(inline)]
pub use bytes::*;

#[doc(inline)]
#[cfg(feature = "alloc")]
pub use self::opt_alloc::*;

#[cfg(not(feature = "alloc"))]
pub use self::opt_fixed::*;

/// docs for opt_alloc::Opt and opt_fixed::Opt
  macro_rules! opt_docs {
    () => {
        r#"Low-level representation of a freshly parsed CoAP Option

Both requests and responses may include a list of one or more
options. For example, the URI in a request is transported in several
options, and metadata that would be carried in an HTTP header in HTTP
is supplied as options as well.

## Option Numbers
This struct just stores data parsed directly from the message on the wire,
and does not compute or store the Option Number.

To get Option [`OptNumber`]s, you can use the iterator extension [`EnumerateOptNumbers`] on a collection of [`Opt`]s.

## `alloc` / no-`alloc`
When crate feature `alloc` is enabled, you can use [`opt_alloc::Opt`] or just `opt::Opt`, which uses heap allocation
for data storage.

When `alloc` is disabled, you must use [`opt_fixed::Opt`] or just `opt::Opt`, which instead has a fixed capacity and
uses stack allocation for data storage.

# Related
- [RFC7252#section-3.1 Option Format](https://datatracker.ietf.org/doc/html/rfc7252#section-3.1)
- [RFC7252#section-5.4 Options](https://datatracker.ietf.org/doc/html/rfc7252#section-5.4)"#};
  }
pub(self) use opt_docs;

/// docs for opt_alloc::Value and opt_fixed::Value
macro_rules! value_docs {
  () => {
      r#"Option Value

# Related
- [RFC7252#section-3.1 Option Format](https://datatracker.ietf.org/doc/html/rfc7252#section-3.1)
- [RFC7252#section-5.4 Options](https://datatracker.ietf.org/doc/html/rfc7252#section-5.4)"#}
}
pub(self) use value_docs;

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
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct OptDelta(pub u16);

impl<T: IntoIterator<Item = u8>> TryConsumeBytes<T> for OptDelta {
  type Error = OptParseError;

  fn try_consume_bytes(bytes: T) -> Result<Self, Self::Error> {
    let mut bytes = bytes.into_iter();
    let first_byte = try_next(&mut bytes)?;
    let delta = first_byte >> 4;
    let delta = opt_len_or_delta(delta, &mut bytes, OptParseError::OptionDeltaReservedValue(15))?;

    Ok(OptDelta(delta))
  }
}

/// Trait for getting the delta from either heap or heapless Opts
pub trait GetOptDelta {
  /// ```
  /// use kwap_msg::*;
  ///
  /// let heaped = opt_alloc::Opt {delta: OptDelta(1), value: opt_alloc::OptValue(vec![])};
  /// let stackd = opt_fixed::Opt::<128> {delta: OptDelta(1), value: opt_fixed::OptValue(arrayvec::ArrayVec::new())};
  ///
  /// assert_eq!(heaped.get_delta(), stackd.get_delta());
  /// ```
  fn get_delta(&self) -> OptDelta;
}

#[cfg(feature = "alloc")]
impl GetOptDelta for opt_alloc::Opt {
  fn get_delta(&self) -> OptDelta {self.delta}
}

impl<const CAP: usize> GetOptDelta for opt_fixed::Opt<CAP> {
  fn get_delta(&self) -> OptDelta {self.delta}
}

/// Creates an iterator which gives the current opt's number as well as the option.
///
/// The iterator returned yields pairs `(i, val)`, where `i` is the [`OptNumber`] and `val` is the Opt returned by the iterator.
pub trait EnumerateOptNumbers<T: GetOptDelta>: Iterator<Item = T> where Self: Sized {
  /// Creates an iterator which gives the current Opt along with its Number.
  ///
  /// ```
  /// use kwap_msg::opt::*;
  ///
  /// let opt_a = Opt {delta: OptDelta(12), value: OptValue(Vec::new())};
  /// let opt_b = Opt {delta: OptDelta(2), value: OptValue(Vec::new())};
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

impl<T: GetOptDelta, I: Iterator<Item = T>> EnumerateOptNumbers<T> for I {
  fn enumerate_option_numbers(self) -> EnumerateOptNumbersIter<T, Self> {
    EnumerateOptNumbersIter {
      number: 0,
      iter: self,
    }
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
