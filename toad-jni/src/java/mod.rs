/// java/lang/*
pub mod lang;

/// java/net/*
pub mod net;

/// java/nio/*
pub mod nio;

/// java/io/*
pub mod io;

/// java/math/*
pub mod math;

/// java/util/*
pub mod util;

/// java/time/*
pub mod time;

mod nullable;

#[doc(inline)]
pub use nullable::Nullable;

mod result;

#[doc(inline)]
pub use result::ResultExt;

mod no_upcast;

#[doc(inline)]
pub use no_upcast::NoUpcast;

mod class;

#[doc(inline)]
pub use class::Class;

mod object;

#[doc(inline)]
pub use object::Object;

mod primitive;

#[doc(inline)]
pub use primitive::Primitive;

mod ty;

#[doc(inline)]
pub use ty::{Signature, Type};

mod function;

#[doc(inline)]
pub use function::{Constructor, Method, StaticMethod};

mod field;
#[doc(inline)]
pub use field::{Field, StaticField};

/// Derive [`crate::java::Object`] for a tuple struct with 1 [`crate::java::lang::Object`] field.
#[macro_export]
macro_rules! object_newtype {
  ($ty:ty) => {
    impl $crate::java::Object for $ty {
      fn upcast<'a, 'e>(_: &'a mut java::Env<'e>, jobj: java::lang::Object) -> Self {
        Self(jobj)
      }

      fn downcast<'a, 'e>(self, _: &'a mut java::Env<'e>) -> java::lang::Object {
        self.0
      }

      fn downcast_ref<'a, 'e>(&'a self, e: &'a mut java::Env<'e>) -> java::lang::Object {
        self.0.downcast_ref(e)
      }
    }
  };
}
pub use object_newtype;

/// Alias for [`jni::JNIEnv`]
pub type Env<'local> = jni::JNIEnv<'local>;

/// Create a new local frame from the global jvm handle
pub fn env<'a>() -> Env<'a> {
  crate::global::jvm().get_env().unwrap()
}
