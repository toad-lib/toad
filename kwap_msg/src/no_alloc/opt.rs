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
