use std::collections::VecDeque;

use crate::java;

/// java/math/BigInteger
pub struct BigInteger(java::lang::Object);

impl BigInteger {
  /// java.math.BigInteger.ONE
  pub fn one<'a>(e: &mut java::Env<'a>) -> Self {
    static ONE: java::StaticField<BigInteger, BigInteger> = java::StaticField::new("ONE");
    ONE.get(e)
  }

  /// java.math.BigInteger.TWO
  pub fn two<'a>(e: &mut java::Env<'a>) -> Self {
    static TWO: java::StaticField<BigInteger, BigInteger> = java::StaticField::new("TWO");
    TWO.get(e)
  }

  /// java.math.BigInteger.TEN
  pub fn ten<'a>(e: &mut java::Env<'a>) -> Self {
    static TEN: java::StaticField<BigInteger, BigInteger> = java::StaticField::new("TEN");
    TEN.get(e)
  }

  /// java.math.BigInteger.ZERO
  pub fn zero<'a>(e: &mut java::Env<'a>) -> Self {
    static ZERO: java::StaticField<BigInteger, BigInteger> = java::StaticField::new("ZERO");
    ZERO.get(e)
  }

  /// Interpret result of [`BigInteger::to_be_bytes`] as a i8
  pub fn to_i8<'a>(&self, e: &mut java::Env<'a>) -> i8 {
    let bytes = self.to_be_bytes::<1>(e);
    i8::from_be_bytes(bytes)
  }

  /// Interpret result of [`BigInteger::to_be_bytes`] as a i16
  pub fn to_i16<'a>(&self, e: &mut java::Env<'a>) -> i16 {
    let bytes = self.to_be_bytes::<2>(e);
    i16::from_be_bytes(bytes)
  }

  /// Interpret result of [`BigInteger::to_be_bytes`] as a i32
  pub fn to_i32<'a>(&self, e: &mut java::Env<'a>) -> i32 {
    let bytes = self.to_be_bytes::<4>(e);
    i32::from_be_bytes(bytes)
  }

  /// Interpret result of [`BigInteger::to_be_bytes`] as a i64
  pub fn to_i64<'a>(&self, e: &mut java::Env<'a>) -> i64 {
    let bytes = self.to_be_bytes::<8>(e);
    i64::from_be_bytes(bytes)
  }

  /// Interpret result of [`BigInteger::to_be_bytes`] as a i128
  pub fn to_i128<'a>(&self, e: &mut java::Env<'a>) -> i128 {
    let bytes = self.to_be_bytes::<16>(e);
    i128::from_be_bytes(bytes)
  }

  /// Extract the raw bytes in big-endian order of this BigInteger, panicking if negative or too big
  pub fn to_be_bytes<'a, const N: usize>(&self, e: &mut java::Env<'a>) -> [u8; N] {
    static TO_BYTE_ARRAY: java::Method<BigInteger, fn() -> Vec<i8>> =
      java::Method::new("toByteArray");

    let mut bytes = VecDeque::from(TO_BYTE_ARRAY.invoke(e, &self));

    let mut byte_array = [0u8; N];

    // if `bytes: VecDeque` is shorter than `N`,
    // this will ensure that `byte_array` is zero-padded,
    // and panic if there are more bytes than `N`

    if let Some((first_nonzero_ix, _)) = bytes.iter().enumerate().find(|(_, b)| **b > 0) {
      bytes.drain(0..first_nonzero_ix).for_each(|_| ());
    }

    bytes.iter()
         .map(|i| {
           // https://docs.oracle.com/en/java/javase/19/docs/api/java.base/java/math/BigInteger.html#toByteArray()
           //
           // toByteArray returns the raw byte representation
           // of the integer, NOT i8s which are the normal
           // interpretation for a java `byte` primitive.
           i8::to_be_bytes(*i)[0]
         })
         .rfold(N - 1, |ix, b| {
           byte_array[ix] = b;
           ix.checked_sub(1).unwrap_or(0)
         });

    byte_array
  }

  /// Create a BigInteger from some bytes, easily gotten
  /// for any signed rust integer (`i8`, `i16`, ..) via `.to_be_bytes()`.
  ///
  /// Technically, this uses `java.math.BigInteger(byte[] bytes)` to
  /// create a `java.math.BigInteger` from an array of bytes that
  /// must represent a signed two's complement integer, in big-endian order.
  pub fn from_be_bytes<'a>(e: &mut java::Env<'a>, bytes: &[u8]) -> Self {
    static CTOR_BYTE_ARRAY: java::Constructor<BigInteger, fn(Vec<i8>)> = java::Constructor::new();
    CTOR_BYTE_ARRAY.invoke(e,
                           bytes.iter()
                                .copied()
                                .map(|u| i8::from_be_bytes(u.to_be_bytes()))
                                .collect())
  }
}

impl java::Class for BigInteger {
  const PATH: &'static str = "java/math/BigInteger";
}

impl java::Object for BigInteger {
  fn upcast<'a, 'e>(_e: &'a mut java::Env<'e>, jobj: java::lang::Object) -> Self {
    Self(jobj)
  }

  fn downcast<'a, 'e>(self, _e: &'a mut java::Env<'e>) -> java::lang::Object {
    self.0
  }

  fn downcast_ref<'a, 'e>(&'a self, e: &'a mut java::Env<'e>) -> java::lang::Object {
    (&self.0).downcast_ref(e)
  }
}
