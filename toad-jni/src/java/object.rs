use java::Type;
use jni::objects::{GlobalRef,
                   JBooleanArray,
                   JByteArray,
                   JCharArray,
                   JDoubleArray,
                   JFloatArray,
                   JIntArray,
                   JLongArray,
                   JObject,
                   JObjectArray,
                   JShortArray,
                   JValue,
                   JValueOwned};

use crate::java;

/// A rust type that can be converted to & from a [`java::lang::Object`]
///
/// notably, this includes all [`java::Primitive`]s (using their wrapper classes).
///
/// For more information, see [`java::Type`]
pub trait Object
  where Self: java::Type
{
  /// Try to interpret an object as `Self`
  fn upcast(e: &mut java::Env, jobj: java::lang::Object) -> Self;

  /// Create an object reference from `self`
  fn downcast(self, e: &mut java::Env) -> java::lang::Object;

  /// Create an object reference from `&self`
  fn downcast_ref(&self, e: &mut java::Env) -> java::lang::Object;

  /// Try to interpret a `JValue` as `Self`
  fn upcast_value_ref<'e>(e: &mut java::Env<'e>, jv: JValue<'e, '_>) -> Self
    where Self: Sized
  {
    java::lang::Object::upcast_value_ref(e, jv).upcast_to::<Self>(e)
  }

  /// Try to interpret a `JValueOwned` as `Self`
  fn upcast_value<'e>(e: &mut java::Env<'e>, jv: JValueOwned<'e>) -> Self
    where Self: Sized
  {
    Self::upcast_value_ref(e, (&jv).into())
  }

  /// Create a `JValueOwned` from `self`
  fn downcast_value<'e>(self, e: &mut java::Env<'e>) -> JValueOwned<'e>
    where Self: Sized
  {
    self.downcast(e).downcast_value(e)
  }
}

impl Object for GlobalRef {
  fn upcast(_: &mut java::Env, jobj: java::lang::Object) -> Self {
    jobj.into()
  }

  fn downcast(self, _: &mut java::Env) -> java::lang::Object {
    self.into()
  }

  fn downcast_ref(&self, e: &mut java::Env) -> java::lang::Object {
    e.new_global_ref(self).unwrap().into()
  }
}

impl<T> Object for Vec<T> where T: java::Object
{
  fn upcast(e: &mut java::Env, jobj: java::lang::Object) -> Self {
    let arr = <&JObjectArray>::from(jobj.as_local());
    let len = e.get_array_length(arr).unwrap() as usize;

    macro_rules! go {
      ($arr:ty => $get_region:ident) => {{
        let arr = <&$arr>::from(jobj.as_local());
        let mut els = Vec::new();
        els.resize(len, Default::default());
        e.$get_region(&arr, 0, &mut els).unwrap();
        els.into_iter()
           .map(|b| {
             let val = b.downcast_value(e);
             T::upcast_value(e, val)
           })
           .collect()
      }};
    }

    match T::SIG {
      | i8::SIG => go!(JByteArray => get_byte_array_region),
      | i16::SIG => go!(JShortArray => get_short_array_region),
      | i32::SIG => go!(JIntArray => get_int_array_region),
      | i64::SIG => go!(JLongArray => get_long_array_region),
      | f32::SIG => go!(JFloatArray => get_float_array_region),
      | f64::SIG => go!(JDoubleArray => get_double_array_region),
      | u16::SIG => go!(JCharArray => get_char_array_region),
      | bool::SIG => {
        let arr = <&JBooleanArray>::from(jobj.as_local());
        let mut els = Vec::new();
        els.resize(len, 0u8);
        e.get_boolean_array_region(arr, 0, &mut els).unwrap();
        els.into_iter()
           .map(|b| {
             let val = (b == jni::sys::JNI_TRUE).downcast_value(e);
             T::upcast_value(e, val)
           })
           .collect()
      },
      | _ if jobj.is_instance_of::<Vec<java::lang::Object>>(e) => {
        let mut vec = Vec::new();

        (0..len).for_each(|ix| {
                  let obj = e.get_object_array_element(arr, ix as i32).unwrap();
                  vec.push(java::lang::Object::from_local(e, obj).upcast_to::<T>(e));
                });

        vec
      },
      | _ => {
        let cls = e.get_object_class(&jobj).unwrap();
        panic!("unknown array type {}",
               java::lang::Object::from_local(e, cls).to_string(e));
      },
    }
  }

  fn downcast(self, e: &mut java::Env) -> java::lang::Object {
    self.downcast_ref(e)
  }

  fn downcast_ref(&self, e: &mut java::Env) -> java::lang::Object {
    macro_rules! go {
      ($new_array:ident, $set_region:ident) => {{
        let slice = self.as_slice();

        // SAFETY: we checked in the match arm that the
        // type contained in this vec is correct for the
        // array type being constructed.
        //
        // The only way this could cause UB is if someone
        // intentionally and maliciously wrote a struct with
        // a Type::SIG matching the primitive type's; it would
        // have to be intentional and malicious because there is
        // no public API to declare non-class `Signature`s.
        let slice = unsafe { core::mem::transmute::<_, &[_]>(slice) };

        let arr = e.$new_array(self.len() as i32).unwrap();
        e.$set_region(&arr, 0, slice).unwrap();
        java::lang::Object::from_local(e, arr)
      }};
    }

    match T::SIG {
      | i8::SIG => go!(new_byte_array, set_byte_array_region),
      | i16::SIG => go!(new_short_array, set_short_array_region),
      | i32::SIG => go!(new_int_array, set_int_array_region),
      | i64::SIG => go!(new_long_array, set_long_array_region),
      | f32::SIG => go!(new_float_array, set_float_array_region),
      | f64::SIG => go!(new_double_array, set_double_array_region),
      | u16::SIG => go!(new_char_array, set_char_array_region),
      | bool::SIG => go!(new_boolean_array, set_boolean_array_region),
      | _ => {
        let arr = e.new_object_array(self.len() as i32, java::lang::Object::SIG, JObject::null())
                   .unwrap();
        let arr_ref = &arr;
        self.iter().enumerate().for_each(|(ix, o)| {
                                 let val = o.downcast_ref(e);
                                 e.set_object_array_element(arr_ref, ix as i32, &val)
                                  .unwrap();
                               });

        java::lang::Object::from_local(e, arr)
      },
    }
  }
}

impl<T> Object for T where T: java::Primitive
{
  fn upcast(e: &mut java::Env, jobj: java::lang::Object) -> Self {
    let w = <T as java::Primitive>::PrimitiveWrapper::upcast(e, jobj);
    T::from_primitive_wrapper(e, w)
  }

  fn downcast(self, e: &mut java::Env) -> java::lang::Object {
    self.to_primitive_wrapper(e).downcast(e)
  }

  fn downcast_ref(&self, e: &mut java::Env) -> java::lang::Object {
    self.to_primitive_wrapper(e).downcast(e)
  }

  fn upcast_value_ref<'e>(_: &mut java::Env<'e>, jv: JValue<'e, '_>) -> Self
    where Self: Sized
  {
    T::from_jvalue_ref(jv)
  }

  fn upcast_value(_: &mut java::Env, jv: JValueOwned) -> Self
    where Self: Sized
  {
    T::from_jvalue(jv)
  }

  fn downcast_value<'e>(self, _: &mut java::Env<'e>) -> JValueOwned<'e>
    where Self: Sized
  {
    self.into_jvalue()
  }
}

impl Object for () {
  fn upcast(_: &mut java::Env, _: java::lang::Object) -> Self {
    ()
  }

  fn downcast(self, e: &mut java::Env) -> java::lang::Object {
    java::lang::Object::from_local(e, JObject::null())
  }

  fn downcast_ref(&self, e: &mut java::Env) -> java::lang::Object {
    ().downcast(e)
  }

  fn upcast_value_ref<'e>(_: &mut java::Env<'e>, jv: JValue<'e, '_>) -> Self
    where Self: Sized
  {
    jv.v().unwrap()
  }

  fn upcast_value<'e>(_: &mut java::Env<'e>, jv: JValueOwned<'e>) -> Self
    where Self: Sized
  {
    jv.v().unwrap()
  }

  fn downcast_value<'e>(self, _: &mut java::Env<'e>) -> JValueOwned<'e>
    where Self: Sized
  {
    JValueOwned::Void
  }
}
