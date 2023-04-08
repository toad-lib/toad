use std::fmt::Display;

use jni::objects::{GlobalRef, JObject};

use super::Class;
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

  /// Get the [`jni::signature::ReturnType`] of a function [`Signature`]
  pub fn return_type(self) -> jni::signature::ReturnType {
    use jni::signature::Primitive::*;
    use jni::signature::ReturnType::*;

    let ret = self.as_str();
    let ret = ret.split(')')
                 .nth(1)
                 .unwrap_or_else(|| panic!("{:?} is not a function signature", self));

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
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(f, "{}", self.as_str())
  }
}

impl AsRef<str> for Signature {
  fn as_ref(&self) -> &str {
    self.as_str()
  }
}

mod type_sealed {
  use jni::objects::GlobalRef;

  use crate::java;

  #[allow(unreachable_pub)]
  pub trait TypeSealed {}
  impl<T> TypeSealed for Vec<T> where T: java::Type {}
  impl<R> TypeSealed for fn() -> R where R: java::Type {}
  impl<A, R> TypeSealed for fn(A) -> R where R: java::Type {}
  impl<A, B, R> TypeSealed for fn(A, B) -> R where R: java::Type {}
  impl<A, B, C, R> TypeSealed for fn(A, B, C) -> R where R: java::Type {}
  impl<A, B, C, D, R> TypeSealed for fn(A, B, C, D) -> R where R: java::Type {}
  impl<A, B, C, D, E, R> TypeSealed for fn(A, B, C, D, E) -> R where R: java::Type {}
  impl<T> TypeSealed for T where T: java::Class {}
  impl TypeSealed for GlobalRef {}
  impl TypeSealed for () {}
  impl TypeSealed for u16 {}
  impl TypeSealed for bool {}
  impl TypeSealed for i8 {}
  impl TypeSealed for i16 {}
  impl TypeSealed for i32 {}
  impl TypeSealed for i64 {}
  impl TypeSealed for f32 {}
  impl TypeSealed for f64 {}
}

/// A type that has a corresponding Java type
///
/// ## Sealed
/// You can't implement this trait directly; instead you can define new
/// [`java::Class`]es, which will then come with [`Type`] implementations for free.
///
/// ## Conversions
/// |rust type|java type|notes|
/// |--|--|--|
/// |`T where T: `[`java::Class`]|fully qualified class path||
/// |[`java::Nullable`]`<T>`|`T::PATH`|[`java::Class`] must be implemented for `T`|
/// |[`java::NoUpcast`]`<T>`|`java::lang::Object`|[`java::Class`] must be implemented for `T`. Used when a method should have the signature of returning `T`, but you would like the object reference without [`java::Object::upcast`]ing.|
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
pub trait Type: type_sealed::TypeSealed {
  /// The signature for this type
  const SIG: Signature;

  /// Determines whether an object is an instance of this type
  fn is_type_of(e: &mut java::Env, o: &JObject) -> bool {
    e.is_instance_of(o, Self::SIG).unwrap()
  }

  /// Get the [`jni`] rep of this type
  fn jni() -> jni::signature::JavaType;
}

impl<T> Type for T where T: java::Class
{
  const SIG: Signature = Signature::class(T::PATH);
  fn jni() -> jni::signature::JavaType {
    jni::signature::JavaType::Object(T::PATH.into())
  }
}

impl Type for GlobalRef {
  const SIG: Signature = java::lang::Object::SIG;
  fn jni() -> jni::signature::JavaType {
    jni::signature::JavaType::Object(java::lang::Object::PATH.into())
  }
}

impl Type for () {
  const SIG: Signature = Signature::empty().push_str("V");
  fn jni() -> jni::signature::JavaType {
    jni::signature::JavaType::Primitive(jni::signature::Primitive::Void)
  }
}

impl Type for u16 {
  const SIG: Signature = Signature::empty().push_str("C");
  fn jni() -> jni::signature::JavaType {
    jni::signature::JavaType::Primitive(jni::signature::Primitive::Char)
  }
}

impl Type for i8 {
  const SIG: Signature = Signature::empty().push_str("B");
  fn jni() -> jni::signature::JavaType {
    jni::signature::JavaType::Primitive(jni::signature::Primitive::Byte)
  }
}

impl Type for i16 {
  const SIG: Signature = Signature::empty().push_str("S");
  fn jni() -> jni::signature::JavaType {
    jni::signature::JavaType::Primitive(jni::signature::Primitive::Short)
  }
}

impl Type for i32 {
  const SIG: Signature = Signature::empty().push_str("I");
  fn jni() -> jni::signature::JavaType {
    jni::signature::JavaType::Primitive(jni::signature::Primitive::Int)
  }
}

impl Type for i64 {
  const SIG: Signature = Signature::empty().push_str("J");
  fn jni() -> jni::signature::JavaType {
    jni::signature::JavaType::Primitive(jni::signature::Primitive::Long)
  }
}

impl Type for f32 {
  const SIG: Signature = Signature::empty().push_str("F");
  fn jni() -> jni::signature::JavaType {
    jni::signature::JavaType::Primitive(jni::signature::Primitive::Float)
  }
}

impl Type for f64 {
  const SIG: Signature = Signature::empty().push_str("D");
  fn jni() -> jni::signature::JavaType {
    jni::signature::JavaType::Primitive(jni::signature::Primitive::Double)
  }
}

impl Type for bool {
  const SIG: Signature = Signature::empty().push_str("Z");
  fn jni() -> jni::signature::JavaType {
    jni::signature::JavaType::Primitive(jni::signature::Primitive::Boolean)
  }
}

impl<T> Type for Vec<T> where T: Type
{
  const SIG: Signature = Signature::array_of(T::SIG);
  fn jni() -> jni::signature::JavaType {
    jni::signature::JavaType::Array(Box::new(T::jni()))
  }
}

impl<R> Type for fn() -> R where R: Type
{
  const SIG: Signature = Signature::function().ret(R::SIG);
  fn jni() -> jni::signature::JavaType {
    jni::signature::JavaType::Method(Box::new(jni::signature::TypeSignature { args: vec![], ret: Self::SIG.return_type() }))
  }
}

impl<R, A> Type for fn(A) -> R
  where R: Type,
        A: Type
{
  const SIG: Signature = Signature::function().concat(A::SIG).ret(R::SIG);
  fn jni() -> jni::signature::JavaType {
    jni::signature::JavaType::Method(Box::new(jni::signature::TypeSignature { args: vec![A::jni()], ret: Self::SIG.return_type() }))
  }
}

impl<R, A, B> Type for fn(A, B) -> R
  where R: Type,
        A: Type,
        B: Type
{
  const SIG: Signature = Signature::function().concat(A::SIG)
                                              .concat(B::SIG)
                                              .ret(R::SIG);
  fn jni() -> jni::signature::JavaType {
    jni::signature::JavaType::Method(Box::new(jni::signature::TypeSignature { args: vec![A::jni(), B::jni()], ret: Self::SIG.return_type() }))
  }
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
  fn jni() -> jni::signature::JavaType {
    jni::signature::JavaType::Method(Box::new(jni::signature::TypeSignature { args: vec![A::jni(), B::jni(), C::jni()], ret: Self::SIG.return_type() }))
  }
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
  fn jni() -> jni::signature::JavaType {
    jni::signature::JavaType::Method(Box::new(jni::signature::TypeSignature { args: vec![A::jni(), B::jni(), C::jni(), D::jni()], ret: Self::SIG.return_type() }))
  }
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
  fn jni() -> jni::signature::JavaType {
    jni::signature::JavaType::Method(Box::new(jni::signature::TypeSignature { args: vec![A::jni(), B::jni(), C::jni(), D::jni(), E::jni()], ret: Self::SIG.return_type() }))
  }
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
