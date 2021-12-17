use alloc::vec::Vec;
use arrayvec::ArrayVec;

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
pub struct Delta(pub u16);

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
pub struct Number(pub u32);

/// Option Value
///
/// # Related
/// - [RFC7252#section-3.1 Option Format](https://datatracker.ietf.org/doc/html/rfc7252#section-3.1)
/// - [RFC7252#section-5.4 Options](https://datatracker.ietf.org/doc/html/rfc7252#section-5.4)
#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub struct Value {
  /// Length of Option Value, in bytes
  pub len: u16,

  /// Option Value data
  ///
  /// The number 65804 comes from u16::MAX + 269;
  /// Option Values may have a length up to this number due to the semantics of Option Length;
  /// see [RFC7252#section-3.1 Option Format](https://datatracker.ietf.org/doc/html/rfc7252#section-3.1)
  /// for more information.
  #[cfg(feature = "alloc")]
  #[cfg_attr(any(feature = "docs", docsrs), doc(cfg(feature = "alloc")))]
  pub data: Vec<u8>,

  /// Option Value data
  ///
  /// The number 65804 comes from u16::MAX + 269; the theoretical max length of a single option.
  ///
  /// Option Values may have a length up to this number due to the semantics of Option Length;
  /// see [RFC7252#section-3.1 Option Format](https://datatracker.ietf.org/doc/html/rfc7252#section-3.1)
  /// for more information.
  #[cfg(any(feature = "docs", not(feature = "alloc")))]
  #[cfg_attr(any(feature = "docs", docsrs), doc(cfg(not(feature = "alloc"))))]
  pub data_fixed: ArrayVec<u8, 65804>,
}

/// Low-level representation of a freshly parsed CoAP Option
///
/// Both requests and responses may include a list of one or more
/// options. For example, the URI in a request is transported in several
/// options, and metadata that would be carried in an HTTP header in HTTP
/// is supplied as options as well.
///
/// This struct just stores data parsed directly from the message on the wire,
/// and does not compute or store the Option Number.
///
/// To get Option [`Number`]s, you can use the iterator extension [`EnumerateOptNumbers`] on a collection of [`Opt`]s.
///
/// # Related
/// - [RFC7252#section-3.1 Option Format](https://datatracker.ietf.org/doc/html/rfc7252#section-3.1)
/// - [RFC7252#section-5.4 Options](https://datatracker.ietf.org/doc/html/rfc7252#section-5.4)
#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub struct Opt {
  /// See [`Delta`]
  pub delta: Delta,
  /// See [`Value`]
  pub value: Value,
}

/// Creates an iterator which gives the current opt's number as well as the option.
///
/// The iterator returned yields pairs `(i, val)`, where `i` is the [`Number`] and `val` is the Opt returned by the iterator.
pub trait EnumerateOptNumbers: Iterator<Item = Opt> where Self: Sized {
  /// Creates an iterator which gives the current Opt along with its Number.
  ///
  /// ```
  /// use kwap_msg::opt::*;
  ///
  /// let opt_a = Opt {delta: Delta(12), value: Value {len: 1, data: Vec::new()}};
  /// let opt_b = Opt {delta: Delta(2), value: Value {len: 1, data: Vec::new()}};
  /// let opts = vec![opt_a.clone(), opt_b.clone()];
  ///
  /// let opt_ns = opts.into_iter()
  ///                  .enumerate_option_numbers()
  ///                  .collect::<Vec<_>>();
  ///
  /// assert_eq!(opt_ns, vec![(Number(12), opt_a), (Number(14), opt_b)])
  /// ```
  fn enumerate_option_numbers(self) -> EnumerateOptNumbersIter<Self>;
}

impl<T: Iterator<Item = Opt>> EnumerateOptNumbers for T {
  fn enumerate_option_numbers(self) -> EnumerateOptNumbersIter<Self> {
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
pub struct EnumerateOptNumbersIter<I: Iterator<Item = Opt>> {
  number: u32,
  iter: I,
}

/// impl Iterator for EnumerateOptNumbersIter
impl<I: Iterator<Item = Opt>> Iterator for EnumerateOptNumbersIter<I> {
  type Item = (Number, Opt);

  fn next(&mut self) -> Option<Self::Item> {
    if let Some(next) = self.iter.next() {
      self.number += u32::from(next.delta.0);
      Some((Number(self.number), next))
    } else {
      None
    }
 }
}
