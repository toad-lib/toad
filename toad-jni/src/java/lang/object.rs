use std::ops::Deref;

use java::ResultExt;
use jni::objects::{GlobalRef, JObject, JValueGen};

use crate::java;

/// `java.lang.Object`
///
/// The bottom type for all [`java::Class`]es, as well as the target
/// for [`java::Object`] casts.
pub struct Object(GlobalRef);

impl Object {
  /// Is this object `instanceof C`?
  pub fn is_instance_of<T>(&self, e: &mut java::Env) -> bool
    where T: java::Type
  {
    T::is_type_of(e, self)
  }

  /// Shorthand for `T::upcast(e, jobj)`
  pub fn upcast_to<T>(self, e: &mut java::Env) -> T
    where T: java::Object
  {
    T::upcast(e, self)
  }

  /// Create a new global reference to the same object
  pub fn new_reference(&self, e: &mut java::Env) -> Self {
    Self(e.new_global_ref(self.as_local()).unwrap_java(e))
  }

  /// Invoke `String toString()`
  pub fn to_string(&self, e: &mut java::Env) -> String {
    static TO_STRING: java::Method<Object, fn() -> String> = java::Method::new("toString");
    TO_STRING.invoke(e, self)
  }

  /// Invoke `boolean equals(Object o)`
  pub fn equals(&self, e: &mut java::Env, other: &Object) -> bool {
    static EQUALS: java::Method<Object, fn(Object) -> bool> = java::Method::new("equals");
    let other = other.new_reference(e);
    EQUALS.invoke(e, self, other)
  }

  /// Convert an object reference to an owned local jvalue
  pub fn to_value<'a>(&self, e: &mut java::Env<'a>) -> JValueGen<JObject<'a>> {
    JValueGen::Object(self.to_local(e))
  }

  /// Convert an object reference to an owned local jobject
  pub fn to_local<'a>(&self, e: &mut java::Env<'a>) -> JObject<'a> {
    e.new_local_ref(&self.0).unwrap_java(e)
  }

  /// Unwrap an object's inner global reference
  pub fn into_global(self) -> GlobalRef {
    self.0
  }

  /// Convert an owned local jvalue to an object
  pub fn from_value<'a, 'b>(e: &mut java::Env<'a>, jv: JValueGen<JObject<'b>>) -> Self
    where 'a: 'b
  {
    Self::from_local(e, jv.l().unwrap())
  }

  /// Convert a borrowed local jvalue to an object
  pub fn from_value_ref<'a, 'b, 'c>(e: &mut java::Env<'a>, jv: JValueGen<&'c JObject<'b>>) -> Self
    where 'a: 'b,
          'c: 'b
  {
    Self::from_local(e, jv.l().unwrap())
  }

  /// Convert a local jobject to an object
  pub fn from_local<'a, 'b, T>(e: &mut java::Env<'a>, t: T) -> Self
    where 'a: 'b,
          T: AsRef<JObject<'b>>
  {
    Self(e.new_global_ref(t.as_ref()).unwrap_java(e))
  }

  /// Convert a global reference to an object
  pub fn from_global(t: GlobalRef) -> Self {
    Self(t)
  }

  /// Convert an object reference to a borrowed local JValue
  pub fn as_value(&self) -> JValueGen<&JObject<'static>> {
    JValueGen::Object(self.as_local())
  }

  /// Convert an object reference to an owned local JObject
  pub fn as_local(&self) -> &JObject<'static> {
    self.0.as_obj()
  }

  /// Get a reference to this object's inner GlobalRef
  pub fn as_global(&self) -> &GlobalRef {
    &self.0
  }
}

impl java::Class for Object {
  const PATH: &'static str = "java/lang/Object";
}

impl java::Object for Object {
  fn upcast(_: &mut java::Env, jobj: java::lang::Object) -> Self {
    jobj
  }

  fn downcast(self, _: &mut java::Env) -> java::lang::Object {
    self
  }

  fn downcast_ref(&self, e: &mut java::Env) -> java::lang::Object {
    Self::from_local(e, self.as_local())
  }

  fn upcast_value_ref<'e>(e: &mut java::Env<'e>, jv: jni::objects::JValue<'e, '_>) -> Self
    where Self: Sized
  {
    Self::from_value_ref(e, jv)
  }

  fn upcast_value<'e>(e: &mut java::Env<'e>, jv: jni::objects::JValueOwned<'e>) -> Self
    where Self: Sized
  {
    Self::from_value(e, jv)
  }

  fn downcast_value<'e>(self, e: &mut java::Env<'e>) -> jni::objects::JValueOwned<'e>
    where Self: Sized
  {
    self.to_value(e)
  }

  fn yield_to_java(&self, e: &mut java::Env) -> jni::sys::jobject {
    self.to_local(e).as_raw()
  }
}

impl From<GlobalRef> for Object {
  fn from(value: GlobalRef) -> Self {
    Object::from_global(value)
  }
}

impl From<Object> for GlobalRef {
  fn from(value: Object) -> Self {
    value.into_global()
  }
}

impl AsRef<JObject<'static>> for Object {
  fn as_ref(&self) -> &JObject<'static> {
    self.as_local()
  }
}

impl Deref for Object {
  type Target = JObject<'static>;

  fn deref(&self) -> &Self::Target {
    self.as_ref()
  }
}
