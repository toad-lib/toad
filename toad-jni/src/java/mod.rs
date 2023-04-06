/// java/lang/*
pub mod lang;

/// java/math/*
pub mod math;

/// java/util/*
pub mod util;

/// java/time/*
pub mod time;

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

/// Alias for [`jni::JNIEnv`]
pub type Env<'local> = jni::JNIEnv<'local>;

/// Create a new local frame from the global jvm handle
pub fn env<'a>() -> Env<'a> {
  crate::global::jvm().get_env().unwrap()
}
