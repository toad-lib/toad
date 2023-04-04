//! Values that can be converted to & from java value pointers

use core::borrow::Borrow;

use jni::objects::{GlobalRef, JObject, JValue, JValueOwned};

use crate::cls::java;
use crate::global;

/// A value that can be created from a borrowed [`JValue`]
pub trait FromJValueRef {
  /// The type yielded by [`FromJValueRef::from_jvalue_ref`]
  type Output<'a>: Borrow<Self>;

  /// Performs the conversion
  fn from_jvalue_ref<'local, 'a>(jv: JValue<'local, 'a>) -> Self::Output<'a>;
}

/// Primitive values that can be wrapped by an Object class, e.g.
/// [`bool`] is [`Primitive::Wrapped`] by [`java::lang::Bool`].
pub trait Primitive {
  /// The Object type that this type may be wrapped with
  type Wrapped: Object;

  /// Perform the conversion
  fn wrap(&self) -> Self::Wrapped;

  /// Perform the inverse conversion, yielding `Self` from `Self::Wrapped`
  fn dewrap(w: Self::Wrapped) -> Self;

  /// Convert an owned java value to [`Self`]
  fn from_jvalue<'a>(jv: JValueOwned<'a>) -> Self;

  /// Convert [`Self`] to an owned java value
  fn into_jvalue<'a>(self) -> JValueOwned<'a>;
}

/// A type that may refer to, or be converted into, a Java Object.
///
/// Note that in strict Java terms, all objects may be values but not all values are objects;
/// the JVM implementation detail to disallow storing stack-allocated primitives in collections
/// is a huge PITA for this crate, so I've used these traits to invert this relationship
/// and implement [`Object`] for all [`Primitive`]s.
///
/// This allows rust type signatures like `ArrayList<i32>` which uses these traits
/// to desugar to a java type signature like `ArrayList<Int>`.
pub trait Object {
  /// Creates an instance of `Self` from a pinned reference to a java object
  fn from_java(jobj: GlobalRef) -> Self;

  /// Convert `Self` back into a java object reference
  fn to_java(self) -> GlobalRef;

  /// Create an instance of [`Self`] from a temporary reference to a java
  /// object
  ///
  /// This simply pins the reference with [`jni::JNIEnv::new_global_ref`],
  /// using [`crate::global::env()`] to obtain a temporary [`jni::JNIEnv`].
  fn from_jobject(jobj: JObject) -> Self
    where Self: Sized
  {
    let e = crate::global::env();
    let o = e.new_global_ref(jobj).unwrap();
    Self::from_java(o)
  }
}

impl Object for GlobalRef {
  fn from_java(s: GlobalRef) -> Self {
    s
  }

  fn to_java(self) -> GlobalRef {
    self
  }
}

impl FromJValueRef for GlobalRef {
  type Output<'a> = GlobalRef;

  fn from_jvalue_ref<'local, 'a>(jv: JValue<'local, 'a>) -> Self::Output<'a> {
    global::env().new_global_ref(jv.l().unwrap()).unwrap()
  }
}

impl<T> Object for T where T: Primitive
{
  fn from_java(s: GlobalRef) -> Self {
    T::dewrap(<T as Primitive>::Wrapped::from_java(s))
  }

  fn to_java(self) -> GlobalRef {
    self.wrap().to_java()
  }
}

impl FromJValueRef for i8 {
  type Output<'a> = i8;

  fn from_jvalue_ref<'local, 'a>(jv: JValue<'local, 'a>) -> i8 {
    jv.b().unwrap()
  }
}

impl Primitive for i8 {
  type Wrapped = java::lang::Byte;

  fn wrap(&self) -> Self::Wrapped {
    java::lang::Byte::new(*self)
  }

  fn dewrap(w: Self::Wrapped) -> Self {
    w.inner()
  }

  fn from_jvalue<'local>(jv: JValueOwned<'local>) -> Self {
    Self::from_jvalue_ref(jv.borrow())
  }

  fn into_jvalue<'local>(self) -> JValueOwned<'local> {
    self.into()
  }
}

impl FromJValueRef for i16 {
  type Output<'a> = i16;

  fn from_jvalue_ref<'local, 'a>(jv: JValue<'local, 'a>) -> i16 {
    jv.s().unwrap()
  }
}

impl Primitive for i16 {
  type Wrapped = java::lang::Short;

  fn wrap(&self) -> Self::Wrapped {
    java::lang::Short::new(*self)
  }

  fn dewrap(w: Self::Wrapped) -> Self {
    w.inner()
  }

  fn from_jvalue<'local>(jv: JValueOwned<'local>) -> Self {
    Self::from_jvalue_ref(jv.borrow())
  }

  fn into_jvalue<'local>(self) -> JValueOwned<'local> {
    self.into()
  }
}

impl FromJValueRef for i32 {
  type Output<'a> = i32;

  fn from_jvalue_ref<'local, 'a>(jv: JValue<'local, 'a>) -> i32 {
    jv.i().unwrap()
  }
}

impl Primitive for i32 {
  type Wrapped = java::lang::Integer;

  fn wrap(&self) -> Self::Wrapped {
    java::lang::Integer::new(*self)
  }

  fn dewrap(w: Self::Wrapped) -> Self {
    w.inner()
  }

  fn from_jvalue<'local>(jv: JValueOwned<'local>) -> Self {
    Self::from_jvalue_ref(jv.borrow())
  }

  fn into_jvalue<'local>(self) -> JValueOwned<'local> {
    self.into()
  }
}

impl FromJValueRef for i64 {
  type Output<'a> = i64;

  fn from_jvalue_ref<'local, 'a>(jv: JValue<'local, 'a>) -> i64 {
    jv.j().unwrap()
  }
}

impl Primitive for i64 {
  type Wrapped = java::lang::Long;

  fn wrap(&self) -> Self::Wrapped {
    java::lang::Long::new(*self)
  }

  fn dewrap(w: Self::Wrapped) -> Self {
    w.inner()
  }

  fn from_jvalue<'local>(jv: JValueOwned<'local>) -> Self {
    Self::from_jvalue_ref(jv.borrow())
  }

  fn into_jvalue<'local>(self) -> JValueOwned<'local> {
    self.into()
  }
}

impl FromJValueRef for f32 {
  type Output<'a> = f32;

  fn from_jvalue_ref<'local, 'a>(jv: JValue<'local, 'a>) -> f32 {
    jv.f().unwrap()
  }
}

impl Primitive for f32 {
  type Wrapped = java::lang::Float;

  fn wrap(&self) -> Self::Wrapped {
    java::lang::Float::new(*self)
  }

  fn dewrap(w: Self::Wrapped) -> Self {
    w.inner()
  }

  fn from_jvalue<'local>(jv: JValueOwned<'local>) -> Self {
    Self::from_jvalue_ref(jv.borrow())
  }

  fn into_jvalue<'local>(self) -> JValueOwned<'local> {
    self.into()
  }
}

impl FromJValueRef for f64 {
  type Output<'a> = f64;

  fn from_jvalue_ref<'local, 'a>(jv: JValue<'local, 'a>) -> f64 {
    jv.d().unwrap()
  }
}

impl Primitive for f64 {
  type Wrapped = java::lang::Double;

  fn wrap(&self) -> Self::Wrapped {
    java::lang::Double::new(*self)
  }

  fn dewrap(w: Self::Wrapped) -> Self {
    w.inner()
  }

  fn from_jvalue<'local>(jv: JValueOwned<'local>) -> Self {
    Self::from_jvalue_ref(jv.borrow())
  }

  fn into_jvalue<'local>(self) -> JValueOwned<'local> {
    self.into()
  }
}

impl FromJValueRef for bool {
  type Output<'a> = bool;

  fn from_jvalue_ref<'local, 'a>(jv: JValue<'local, 'a>) -> bool {
    jv.z().unwrap()
  }
}

impl Primitive for bool {
  type Wrapped = java::lang::Bool;

  fn wrap(&self) -> Self::Wrapped {
    java::lang::Bool::new(*self)
  }

  fn dewrap(w: Self::Wrapped) -> Self {
    w.inner()
  }

  fn from_jvalue<'local>(jv: JValueOwned<'local>) -> Self {
    Self::from_jvalue_ref(jv.borrow())
  }

  fn into_jvalue<'local>(self) -> JValueOwned<'local> {
    self.into()
  }
}
