use std::collections::VecDeque;

use jni::objects::{GlobalRef, JByteArray, JValueGen};
use jni::JNIEnv;

use crate::convert::Object;
use crate::{convert, Sig};

/// java/math/BigInteger
pub struct BigInteger(GlobalRef);

impl BigInteger {
  /// java/math/BigInteger
  pub const PATH: &'static str = "java/math/BigInteger";

  /// java/math/BigInteger.toByteArray
  pub const TO_BYTE_ARRAY: Sig = Sig::new().returning(Sig::array_of(Sig::BYTE));

  /// java/math/BigInteger(byte[])
  pub const CTOR_BYTE_ARRAY: Sig = Sig::new().arg(Sig::array_of(Sig::BYTE))
                                             .returning(Sig::VOID);

  /// java/math/BigInteger.ONE
  pub const ONE: Sig = Sig::class(Self::PATH);

  /// java/math/BigInteger.TWO
  pub const TWO: Sig = Sig::class(Self::PATH);

  /// java/math/BigInteger.TEN
  pub const TEN: Sig = Sig::class(Self::PATH);

  /// java/math/BigInteger.ZERO
  pub const ZERO: Sig = Sig::class(Self::PATH);

  /// java.math.BigInteger.ONE
  pub fn one<'a>(e: &mut JNIEnv<'a>) -> Self {
    let obj = e.get_static_field(Self::PATH, "ONE", Self::ONE)
               .unwrap()
               .l()
               .unwrap();
    Self(e.new_global_ref(obj).unwrap())
  }

  /// java.math.BigInteger.TWO
  pub fn two<'a>(e: &mut JNIEnv<'a>) -> Self {
    let obj = e.get_static_field(Self::PATH, "TWO", Self::TWO)
               .unwrap()
               .l()
               .unwrap();
    Self(e.new_global_ref(obj).unwrap())
  }

  /// java.math.BigInteger.TEN
  pub fn ten<'a>(e: &mut JNIEnv<'a>) -> Self {
    let obj = e.get_static_field(Self::PATH, "TEN", Self::TEN)
               .unwrap()
               .l()
               .unwrap();
    Self(e.new_global_ref(obj).unwrap())
  }

  /// java.math.BigInteger.ZERO
  pub fn zero<'a>(e: &mut JNIEnv<'a>) -> Self {
    let obj = e.get_static_field(Self::PATH, "ZERO", Self::ZERO)
               .unwrap()
               .l()
               .unwrap();
    Self(e.new_global_ref(obj).unwrap())
  }

  /// Interpret result of [`BigInteger::to_be_bytes`] as a i8
  pub fn to_i8<'a>(&self, e: &mut JNIEnv<'a>) -> i8 {
    let bytes = self.to_be_bytes::<1>(e);
    i8::from_be_bytes(bytes)
  }

  /// Interpret result of [`BigInteger::to_be_bytes`] as a i16
  pub fn to_i16<'a>(&self, e: &mut JNIEnv<'a>) -> i16 {
    let bytes = self.to_be_bytes::<2>(e);
    i16::from_be_bytes(bytes)
  }

  /// Interpret result of [`BigInteger::to_be_bytes`] as a i32
  pub fn to_i32<'a>(&self, e: &mut JNIEnv<'a>) -> i32 {
    let bytes = self.to_be_bytes::<4>(e);
    i32::from_be_bytes(bytes)
  }

  /// Interpret result of [`BigInteger::to_be_bytes`] as a i64
  pub fn to_i64<'a>(&self, e: &mut JNIEnv<'a>) -> i64 {
    let bytes = self.to_be_bytes::<8>(e);
    i64::from_be_bytes(bytes)
  }

  /// Interpret result of [`BigInteger::to_be_bytes`] as a i128
  pub fn to_i128<'a>(&self, e: &mut JNIEnv<'a>) -> i128 {
    let bytes = self.to_be_bytes::<16>(e);
    i128::from_be_bytes(bytes)
  }

  /// Extract the raw bytes in big-endian order of this BigInteger, panicking if negative or too big
  pub fn to_be_bytes<'a, const N: usize>(&self, e: &mut JNIEnv<'a>) -> [u8; N] {
    let jbyte_array: JByteArray<'a> =
      e.call_method(&self.0.as_obj(), "toByteArray", Self::TO_BYTE_ARRAY, &[])
       .unwrap()
       .l()
       .unwrap()
       .try_into()
       .unwrap();

    let mut bytes = VecDeque::new();
    let len = e.get_array_length(&jbyte_array).unwrap() as usize;
    bytes.resize(len, 0i8);

    e.get_byte_array_region(&jbyte_array, 0, bytes.make_contiguous())
     .unwrap();

    // https://docs.oracle.com/en/java/javase/19/docs/api/java.base/java/math/BigInteger.html#toByteArray()
    //
    // BigInt.toByteArray actually returns the raw byte representation of the integer, NOT
    // two's complement `byte`s as the type signature would lead you to believe.
    //
    // To interpret these bytes as i8 is incorrect.
    //
    // In order to get around pesky length issues I zero-pad the beginning by
    // copying the vec's contents into the beginning of the array reversed,
    // then reverse the array.
    let mut arr = [0u8; N];
    bytes.iter()
         .map(|i| i8::to_be_bytes(*i)[0])
         .rev()
         .enumerate()
         .for_each(|(ix, b)| arr[ix] = b);
    arr.reverse();
    arr
  }

  /// Create a `java.math.BigInteger` from raw big-endian two's complement bytes
  ///
  /// `to_be_bytes` may be invoked on any signed rust integer (`i8`, `i16`, ..)
  /// and passed directly as an argument to this function.
  pub fn from_be_bytes<'a>(e: &mut JNIEnv<'a>, bytes: &[u8]) -> Self {
    let byte_array = e.byte_array_from_slice(bytes).unwrap();
    let obj = e.new_object(Self::PATH, Self::CTOR_BYTE_ARRAY, &[(&byte_array).into()])
               .unwrap();
    Self(e.new_global_ref(obj).unwrap())
  }
}

impl Object for BigInteger {
  fn from_java(jobj: GlobalRef) -> Self {
    Self(jobj)
  }

  fn to_java(self) -> GlobalRef {
    self.0
  }
}
