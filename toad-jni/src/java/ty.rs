use std::fmt::Display;

use jni::objects::{GlobalRef, JObject};

use crate::java;

/// Provides strongly-typed JVM type signature strings at compile-time
///
/// A `Signature` can be obtained for all [`Type`] via [`Signature::of`]:
/// ```
/// use toad_jni::java;
///
/// assert_eq!(java::Signature::of::<i32>().as_str(), "I");
/// assert_eq!(java::Signature::of::<()>().as_str(), "V");
///
/// type SumBigInts = fn(Vec<java::math::BigInteger>) -> java::math::BigInteger;
/// assert_eq!(java::Signature::of::<SumBigInts>().as_str(),
///            "([Ljava/math/BigInteger;)Ljava/math/BigInteger;");
/// ```
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Signature {
  bytes: [u8; 256],
  len: usize,
  finished: bool,
}

impl Signature {
  const CLASS_PATH_OPEN: Self = Self::empty().push_str("L");
  const CLASS_PATH_CLOSE: Self = Self::empty().push_str(";");
  const ARRAY_OF: Self = Self::empty().push_str("[");
  const ARGS_OPEN: Self = Self::empty().push_str("(");
  const ARGS_CLOSE: Self = Self::empty().push_str(")");

  /// Get the `Signature` instance for type `T`
  pub const fn of<T>() -> Self
    where T: Type
  {
    T::SIG
  }

  pub(crate) fn return_type(self) -> jni::signature::ReturnType {
    use jni::signature::Primitive::*;
    use jni::signature::ReturnType::*;

    let ret = self.as_str();
    let ret = ret.split(")")
                 .skip(1)
                 .next()
                 .expect(&format!("{:?} is not a function signature", self));

    if ret.starts_with(Self::ARRAY_OF.as_str()) {
      Array
    } else if ret.starts_with(Self::CLASS_PATH_OPEN.as_str()) {
      Object
    } else {
      Primitive(match Signature::empty().push_str(ret) {
                  | <()>::SIG => Void,
                  | bool::SIG => Boolean,
                  | u16::SIG => Char,
                  | i8::SIG => Byte,
                  | i16::SIG => Short,
                  | i32::SIG => Int,
                  | i64::SIG => Long,
                  | f32::SIG => Float,
                  | f64::SIG => Double,
                  | _ => unreachable!(),
                })
    }
  }

  const fn empty() -> Self {
    Self { bytes: [0; 256],
           len: 0,
           finished: false }
  }

  pub(crate) const fn function() -> Self {
    Self::empty().concat(Self::ARGS_OPEN)
  }

  const fn array_of(t: Self) -> Self {
    Self::empty().concat(Self::ARRAY_OF).concat(t)
  }

  const fn class(path: &'static str) -> Self {
    Self::empty().concat(Self::CLASS_PATH_OPEN)
                 .push_str(path)
                 .concat(Self::CLASS_PATH_CLOSE)
  }

  const fn next(&self) -> usize {
    self.len
  }

  pub(crate) const fn concat(mut self, other: Signature) -> Self {
    let mut i = 0;
    loop {
      if i == other.len {
        break;
      }
      self = self.push_byte(other.bytes[i]);
      i += 1;
    }

    self
  }

  const fn push_byte(mut self, b: u8) -> Self {
    if self.finished {
      panic!("cannot modify Sig after invoking .ret()")
    }
    let n = self.next();
    self.bytes[n] = b;
    self.len += 1;
    self
  }

  const fn push_str(mut self, s: &str) -> Self {
    let mut i = 0;
    loop {
      if i == s.len() {
        break;
      }

      let b = s.as_bytes()[i];
      self = self.push_byte(b);

      i += 1;
    }

    self
  }

  pub(crate) const fn ret(mut self, ret: Self) -> Self {
    self = self.concat(Self::ARGS_CLOSE).concat(ret);
    self.finished = true;
    self
  }

  /// Convert a [`Signature`] reference to [`str`]
  pub fn as_str(&self) -> &str {
    match core::str::from_utf8(&self.bytes[0..self.len]) {
      | Ok(s) => s,
      | _ => unreachable!(),
    }
  }
}

impl Display for Signature {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.as_str())
  }
}

impl AsRef<str> for Signature {
  fn as_ref(&self) -> &str {
    self.as_str()
  }
}

/// A rust type with a corresponding java type
///
/// ## Conversions
/// |rust type|java type|notes|
/// |--|--|--|
/// |`T where T: `[`java::Class`]|fully qualified class path||
/// |[`java::lang::Object`]|`java.lang.Object`||
/// |[`Vec`]`<T>`|`T[]`|`T` must be [`java::Type`]|
/// |[`String`]|`java.lang.String`|[`java::Class`] and [`java::Object`] implemented for [`String`]|
/// |`()`|`void`||
/// |`u16`|`char`||
/// |`i8`|`byte`||
/// |`i16`|`short`||
/// |`i32`|`int`||
/// |`i64`|`long`||
/// |`f32`|`float`||
/// |`f64`|`double`||
/// |`fn(T,*) -> R`|corresponding java type signature|all argument types and return types must be [`java::Type`]|
pub trait Type {
  /// The signature for this type
  const SIG: Signature;

  /// Determines whether an object is an instance of this type
  fn is_type_of<'a, 'b>(e: &mut java::Env<'a>, o: &JObject<'b>) -> bool {
    e.is_instance_of(o, Self::SIG).unwrap()
  }
}

impl<T> Type for T where T: java::Class
{
  const SIG: Signature = Signature::class(T::PATH);
}

impl Type for GlobalRef {
  const SIG: Signature = java::lang::Object::SIG;
}

impl Type for () {
  const SIG: Signature = Signature::empty().push_str("V");
}

impl Type for u16 {
  const SIG: Signature = Signature::empty().push_str("C");
}

impl Type for i8 {
  const SIG: Signature = Signature::empty().push_str("B");
}

impl Type for i16 {
  const SIG: Signature = Signature::empty().push_str("S");
}

impl Type for i32 {
  const SIG: Signature = Signature::empty().push_str("I");
}

impl Type for i64 {
  const SIG: Signature = Signature::empty().push_str("J");
}

impl Type for f32 {
  const SIG: Signature = Signature::empty().push_str("F");
}

impl Type for f64 {
  const SIG: Signature = Signature::empty().push_str("D");
}

impl Type for bool {
  const SIG: Signature = Signature::empty().push_str("Z");
}

impl<T> Type for Vec<T> where T: Type
{
  const SIG: Signature = Signature::array_of(T::SIG);
}

impl<R> Type for fn() -> R where R: Type
{
  const SIG: Signature = Signature::function().ret(R::SIG);
}

impl<R, A> Type for fn(A) -> R
  where R: Type,
        A: Type
{
  const SIG: Signature = Signature::function().concat(A::SIG).ret(R::SIG);
}

impl<R, A, B> Type for fn(A, B) -> R
  where R: Type,
        A: Type,
        B: Type
{
  const SIG: Signature = Signature::function().concat(A::SIG)
                                              .concat(B::SIG)
                                              .ret(R::SIG);
}

impl<R, A, B, C> Type for fn(A, B, C) -> R
  where R: Type,
        A: Type,
        B: Type,
        C: Type
{
  const SIG: Signature = Signature::function().concat(A::SIG)
                                              .concat(B::SIG)
                                              .concat(C::SIG)
                                              .ret(R::SIG);
}

impl<R, A, B, C, D> Type for fn(A, B, C, D) -> R
  where R: Type,
        A: Type,
        B: Type,
        C: Type,
        D: Type
{
  const SIG: Signature = Signature::function().concat(A::SIG)
                                              .concat(B::SIG)
                                              .concat(C::SIG)
                                              .concat(D::SIG)
                                              .ret(R::SIG);
}

impl<R, A, B, C, D, E> Type for fn(A, B, C, D, E) -> R
  where R: Type,
        A: Type,
        B: Type,
        C: Type,
        D: Type,
        E: Type
{
  const SIG: Signature = Signature::function().concat(A::SIG)
                                              .concat(B::SIG)
                                              .concat(C::SIG)
                                              .concat(D::SIG)
                                              .concat(E::SIG)
                                              .ret(R::SIG);
}

#[cfg(test)]
pub mod test {
  use crate::java::Signature;

  #[test]
  fn return_type() {
    use jni::signature::Primitive::*;
    use jni::signature::ReturnType::*;

    assert_eq!(Signature::of::<fn()>().return_type(), Primitive(Void));
    assert_eq!(Signature::of::<fn(String) -> String>().return_type(),
               Object);
    assert_eq!(Signature::of::<fn(String) -> Vec<String>>().return_type(),
               Array);
    assert_eq!(Signature::of::<fn() -> u16>().return_type(),
               Primitive(Char));
    assert_eq!(Signature::of::<fn() -> bool>().return_type(),
               Primitive(Boolean));
    assert_eq!(Signature::of::<fn() -> i8>().return_type(), Primitive(Byte));
    assert_eq!(Signature::of::<fn() -> i16>().return_type(),
               Primitive(Short));
    assert_eq!(Signature::of::<fn() -> i32>().return_type(), Primitive(Int));
    assert_eq!(Signature::of::<fn() -> i64>().return_type(),
               Primitive(Long));
    assert_eq!(Signature::of::<fn() -> f32>().return_type(),
               Primitive(Float));
    assert_eq!(Signature::of::<fn() -> f64>().return_type(),
               Primitive(Double));
  }
}
