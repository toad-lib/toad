use jni::objects::{JValue, JValueOwned};

use crate::java;

/// Primitive java values that can be cheaply converted to / from [`JValue`]
/// and can be wrapped by an Object class.
pub trait Primitive
  where Self: java::Type + Sized + Copy
{
  /// The Object type that this type may be wrapped with
  type PrimitiveWrapper: java::Class;

  /// Create a new instance of [`Self::PrimitiveWrapper`] from a copy of `self`
  fn to_primitive_wrapper(&self, e: &mut java::Env) -> Self::PrimitiveWrapper;

  /// Perform the inverse conversion, yielding `Self` from `Self::PrimitiveWrapper`
  fn from_primitive_wrapper(e: &mut java::Env, w: Self::PrimitiveWrapper) -> Self;

  /// Convert a local JValue reference to `Self`
  fn from_jvalue_ref(jv: JValue) -> Self;

  /// Convert an owned local JValue to Self
  fn from_jvalue(jv: JValueOwned) -> Self;

  /// Convert self to a JValue
  fn into_jvalue<'local>(self) -> JValueOwned<'local>;
}

impl Primitive for i8 {
  type PrimitiveWrapper = java::lang::Byte;

  fn to_primitive_wrapper(&self, e: &mut java::Env) -> Self::PrimitiveWrapper {
    java::lang::Byte::new(e, *self)
  }

  fn from_primitive_wrapper(e: &mut java::Env, w: Self::PrimitiveWrapper) -> Self {
    w.inner(e)
  }

  fn from_jvalue_ref(jv: JValue) -> i8 {
    jv.b().unwrap()
  }

  fn from_jvalue(jv: JValueOwned) -> Self {
    jv.b().unwrap()
  }

  fn into_jvalue<'local>(self) -> JValueOwned<'local> {
    self.into()
  }
}

impl Primitive for u16 {
  type PrimitiveWrapper = java::lang::Char;

  fn to_primitive_wrapper(&self, e: &mut java::Env) -> Self::PrimitiveWrapper {
    java::lang::Char::new(e, *self)
  }

  fn from_primitive_wrapper(e: &mut java::Env, w: Self::PrimitiveWrapper) -> Self {
    w.inner(e)
  }

  fn from_jvalue_ref(jv: JValue) -> u16 {
    jv.c().unwrap()
  }

  fn from_jvalue(jv: JValueOwned) -> Self {
    Self::from_jvalue_ref((&jv).into())
  }

  fn into_jvalue<'local>(self) -> JValueOwned<'local> {
    self.into()
  }
}

impl Primitive for i16 {
  type PrimitiveWrapper = java::lang::Short;

  fn to_primitive_wrapper(&self, e: &mut java::Env) -> Self::PrimitiveWrapper {
    java::lang::Short::new(e, *self)
  }

  fn from_primitive_wrapper(e: &mut java::Env, w: Self::PrimitiveWrapper) -> Self {
    w.inner(e)
  }

  fn from_jvalue_ref(jv: JValue) -> i16 {
    jv.s().unwrap()
  }

  fn from_jvalue(jv: JValueOwned) -> Self {
    Self::from_jvalue_ref((&jv).into())
  }

  fn into_jvalue<'local>(self) -> JValueOwned<'local> {
    self.into()
  }
}

impl Primitive for i32 {
  type PrimitiveWrapper = java::lang::Integer;

  fn to_primitive_wrapper(&self, e: &mut java::Env) -> Self::PrimitiveWrapper {
    java::lang::Integer::new(e, *self)
  }

  fn from_primitive_wrapper(e: &mut java::Env, w: Self::PrimitiveWrapper) -> Self {
    w.inner(e)
  }

  fn from_jvalue_ref(jv: JValue) -> i32 {
    jv.i().unwrap()
  }

  fn from_jvalue(jv: JValueOwned) -> Self {
    Self::from_jvalue_ref((&jv).into())
  }

  fn into_jvalue<'local>(self) -> JValueOwned<'local> {
    self.into()
  }
}

impl Primitive for i64 {
  type PrimitiveWrapper = java::lang::Long;

  fn to_primitive_wrapper(&self, e: &mut java::Env) -> Self::PrimitiveWrapper {
    java::lang::Long::new(e, *self)
  }

  fn from_primitive_wrapper(e: &mut java::Env, w: Self::PrimitiveWrapper) -> Self {
    w.inner(e)
  }

  fn from_jvalue_ref(jv: JValue) -> i64 {
    jv.j().unwrap()
  }

  fn from_jvalue(jv: JValueOwned) -> Self {
    Self::from_jvalue_ref((&jv).into())
  }

  fn into_jvalue<'local>(self) -> JValueOwned<'local> {
    self.into()
  }
}

impl Primitive for f32 {
  type PrimitiveWrapper = java::lang::Float;

  fn to_primitive_wrapper(&self, e: &mut java::Env) -> Self::PrimitiveWrapper {
    java::lang::Float::new(e, *self)
  }

  fn from_primitive_wrapper(e: &mut java::Env, w: Self::PrimitiveWrapper) -> Self {
    w.inner(e)
  }

  fn from_jvalue_ref(jv: JValue) -> f32 {
    jv.f().unwrap()
  }

  fn from_jvalue(jv: JValueOwned) -> Self {
    Self::from_jvalue_ref((&jv).into())
  }

  fn into_jvalue<'local>(self) -> JValueOwned<'local> {
    self.into()
  }
}

impl Primitive for f64 {
  type PrimitiveWrapper = java::lang::Double;

  fn to_primitive_wrapper(&self, e: &mut java::Env) -> Self::PrimitiveWrapper {
    java::lang::Double::new(e, *self)
  }

  fn from_primitive_wrapper(e: &mut java::Env, w: Self::PrimitiveWrapper) -> Self {
    w.inner(e)
  }

  fn from_jvalue_ref(jv: JValue) -> f64 {
    jv.d().unwrap()
  }

  fn from_jvalue(jv: JValueOwned) -> Self {
    Self::from_jvalue_ref((&jv).into())
  }

  fn into_jvalue<'local>(self) -> JValueOwned<'local> {
    self.into()
  }
}

impl Primitive for bool {
  type PrimitiveWrapper = java::lang::Bool;

  fn to_primitive_wrapper(&self, e: &mut java::Env) -> Self::PrimitiveWrapper {
    java::lang::Bool::new(e, *self)
  }

  fn from_primitive_wrapper(e: &mut java::Env, w: Self::PrimitiveWrapper) -> Self {
    w.inner(e)
  }

  fn from_jvalue_ref(jv: JValue) -> bool {
    jv.z().unwrap()
  }

  fn from_jvalue(jv: JValueOwned) -> Self {
    Self::from_jvalue_ref((&jv).into())
  }

  fn into_jvalue<'local>(self) -> JValueOwned<'local> {
    self.into()
  }
}
