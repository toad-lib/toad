//! High-level type system for Java <> Rust FFI
//!
//! ## Types
//! All java type signatures can be represented by rust types
//! that implement the [`crate::java::Type`] trait, which is automatically
//! implemented for all [`crate::java::Class`]es.
//!
//! ## Classes
//! Classes are represented in `toad_jni` by implementing 2 traits:
//! * [`crate::java::Class`]
//! * [`crate::java::Object`] (see also [`crate::java::object_newtype`])
//!
//! ### Fields and Methods
//! There are several high-level lens-style structs for interacting with fields, methods and constructors:
//! * [`crate::java::Constructor`]
//! * [`crate::java::StaticField`]
//! * [`crate::java::StaticMethod`]
//! * [`crate::java::Field`]
//! * [`crate::java::Method`]
//!
//! All of these types use [`crate::java::Type`] to transform nice Rust types into the corresponding
//! JVM type signatures.
//!
//! For example, the `StaticMethod` representation of [`java.lang.String.format(String, ..Object)`](https://docs.oracle.com/en/java/javase/19/docs/api/java.base/java/lang/String.html#format(java.lang.String,java.lang.Object...))
//! would be:
//! ```rust,no_run
//! use toad_jni::java::lang::Object;
//! use toad_jni::java::StaticMethod;
//!
//! static STRING_FORMAT: StaticMethod<String, fn(String, Vec<Object>) -> String> =
//!   StaticMethod::new("format");
//! ```
//!
//! It is recommended that these structs are stored in local `static` variables so that they can cache
//! the internal JNI IDs of the class and methods, but this is not required.
//!
//! ### Example
//! Consider the following java class:
//! ```java
//! package com.foo.bar;
//!
//! public class Foo {
//!   public final static long NUMBER = 123;
//!   public String bingus = "bingus";
//!
//!   public Foo() { }
//!
//!   public static String bar() {
//!     return "bar";
//!   }
//!
//!   public void setBingus(String newBingus) {
//!     this.bingus = newBingus;
//!   }
//! }
//! ```
//!
//! A Rust API to this class would look like:
//! ```rust,no_run
//! use toad_jni::java;
//!
//! pub struct Foo(java::lang::Object);
//!
//! java::object_newtype!(Foo);
//!
//! impl java::Class for Foo {
//!   const PATH: &'static str = "com/foo/bar/Foo";
//! }
//!
//! impl Foo {
//!   pub fn new(e: &mut java::Env) -> Self {
//!     static CTOR: java::Constructor<Foo, fn()> = java::Constructor::new();
//!     CTOR.invoke(e)
//!   }
//!
//!   pub fn number(e: &mut java::Env) -> i64 {
//!     static NUMBER: java::StaticField<Foo, i64> = java::StaticField::new("NUMBER");
//!     NUMBER.get(e)
//!   }
//!
//!   pub fn bar(e: &mut java::Env) -> String {
//!     static BAR: java::StaticMethod<Foo, fn() -> String> = java::StaticMethod::new("bar");
//!     BAR.invoke(e)
//!   }
//!
//!   pub fn bingus(&self, e: &mut java::Env) -> String {
//!     static BINGUS: java::Field<Foo, String> = java::Field::new("bingus");
//!     BINGUS.get(e, self)
//!   }
//!
//!   pub fn set_bingus(&self, e: &mut java::Env, s: String) {
//!     static SET_BINGUS: java::Method<Foo, fn(String)> = java::Method::new("setBingus");
//!     SET_BINGUS.invoke(e, self, s)
//!   }
//! }
//! ```

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

/// Derive [`crate::java::Object`] for a tuple struct with 1 [`crate::java::lang::Object`] field.
#[macro_export]
macro_rules! object_newtype {
  ($ty:ty) => {
    impl $crate::java::Object for $ty {
      fn upcast<'a, 'e>(_: &'a mut java::Env<'e>, jobj: java::lang::Object) -> Self {
        Self(jobj)
      }

      fn downcast<'a, 'e>(self, e: &'a mut java::Env<'e>) -> java::lang::Object {
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
