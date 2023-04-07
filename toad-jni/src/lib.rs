//! High-level wrapper of [`jni`], making Java & Rust FFI easy & fun
//!
//! ## Globals
//! [`toad_jni::global`](https://docs.rs/toad-jni/latest/toad_jni/global/index.html) offers the option to use a global JVM handle ([`toad_jni::global::jvm()`](https://docs.rs/toad-jni/latest/toad_jni/global/fn.jvm.html) set with [`toad_jni::global::init()`](https://docs.rs/toad-jni/latest/toad_jni/global/fn.init.html)).
//!
//! Using the JVM global is completely optional, **unless** you plan to use Rust trait impls such as [`IntoIterator`]
//! on [`toad_jni::java::util::ArrayList`](https://docs.rs/toad-jni/latest/toad_jni/java/util/struct.ArrayList.html).
//!
//! ## Types
//! All java type signatures can be represented by rust types
//! that implement the [`toad_jni::java::Type`](https://docs.rs/toad-jni/latest/toad_jni/java/trait.Type.html) trait, which is automatically
//! implemented for all [`toad_jni::java::Class`](https://docs.rs/toad-jni/latest/toad_jni/java/trait.Class.html)es.
//!
//! ## Classes
//! Classes are represented in `toad_jni` by implementing 2 traits:
//! * [`toad_jni::java::Class`](https://docs.rs/toad-jni/latest/toad_jni/java/trait.Class.html)
//! * [`toad_jni::java::Object`](https://docs.rs/toad-jni/latest/toad_jni/java/trait.Object.html) (see also [`toad_jni::java::object_newtype`](https://docs.rs/toad-jni/latest/toad_jni/java/macro.object_newtype.html))
//!
//! ### Fields and Methods
//! There are several high-level lens-style structs for interacting with fields, methods and constructors:
//! * [`toad_jni::java::Constructor`](https://docs.rs/toad-jni/latest/toad_jni/java/struct.Constructor.html)
//! * [`toad_jni::java::StaticField`](https://docs.rs/toad-jni/latest/toad_jni/java/struct.StaticField.html)
//! * [`toad_jni::java::StaticMethod`](https://docs.rs/toad-jni/latest/toad_jni/java/struct.StaticMethod.html)
//! * [`toad_jni::java::Field`](https://docs.rs/toad-jni/latest/toad_jni/java/struct.Field.html)
//! * [`toad_jni::java::Method`](https://docs.rs/toad-jni/latest/toad_jni/java/struct.Method.html)
//!
//! All of these types use [`toad_jni::java::Type`](https://docs.rs/toad-jni/latest/toad_jni/java/trait.Type.html) to transform nice Rust types into the corresponding
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

// docs
#![doc(html_root_url = "https://docs.rs/toad-jni/0.4.1")]
#![cfg_attr(any(docsrs, feature = "docs"), feature(doc_cfg))]
// -
// style
#![allow(clippy::unused_unit)]
// -
// deny
#![deny(missing_docs)]
#![deny(missing_copy_implementations)]
// -
// warnings
#![cfg_attr(not(test), warn(unreachable_pub))]
// -
// features

/// java language features and class shims
pub mod java;

/// Global JVM handles
pub mod global {
  use jni::{InitArgsBuilder, JavaVM};

  static mut JVM: Option<JavaVM> = None;

  /// Initialize the global jvm handle with an existing handle
  pub fn init_with(jvm: JavaVM) {
    unsafe {
      JVM = Some(jvm);
    }
  }

  /// Initialize the global jvm handle by creating a new handle
  pub fn init() {
    unsafe {
      let args = InitArgsBuilder::new().build().unwrap();
      JVM = Some(JavaVM::new(args).unwrap());
    }

    jvm().attach_current_thread_permanently().unwrap();
  }

  /// Get a reference to the global jvm handle
  pub fn jvm() -> &'static mut JavaVM {
    unsafe { JVM.as_mut().unwrap() }
  }
}

#[cfg(test)]
mod test {
  use std::sync::Once;

  use java::Primitive;
  use toad_jni::java;

  pub use crate as toad_jni;

  static INIT: Once = Once::new();
  pub fn init() {
    INIT.call_once(|| {
          toad_jni::global::init();
        });

    toad_jni::global::jvm().attach_current_thread_permanently()
                           .unwrap();
  }

  #[test]
  fn init_works() {
    init();
  }

  #[test]
  fn prim_wrappers() {
    init();
    let mut e = java::env();
    let e = &mut e;

    let i = (32i8).to_primitive_wrapper(e);
    assert_eq!(i8::from_primitive_wrapper(e, i), 32i8);
  }

  #[test]
  fn test_arraylist() {
    init();
    assert_eq!(vec![1i8, 2, 3, 4].into_iter()
                                 .collect::<java::util::ArrayList<i8>>()
                                 .into_iter()
                                 .collect::<Vec<i8>>(),
               vec![1, 2, 3, 4])
  }

  #[test]
  fn test_optional() {
    init();

    let mut e = java::env();

    let o = java::util::Optional::of(&mut e, 12i32);
    assert_eq!(o.to_option(&mut e).unwrap(), 12);

    let o = java::util::Optional::<i32>::empty(&mut e);
    assert!(o.is_empty(&mut e));
  }

  #[test]
  fn test_time() {
    init();

    let mut e = java::env();

    let o = java::time::Duration::of_millis(&mut e, 1000);
    assert_eq!(o.to_millis(&mut e), 1000);
  }

  #[test]
  fn test_bigint() {
    init();

    let mut e = java::env();
    let e = &mut e;

    type BigInt = java::math::BigInteger;

    let bi = BigInt::from_be_bytes(e, &i128::MAX.to_be_bytes());
    assert_eq!(bi.to_i128(e), i128::MAX);

    let bi = BigInt::from_be_bytes(e, &i8::MAX.to_be_bytes());
    assert_eq!(bi.to_i128(e), i8::MAX.into());

    let bi = BigInt::from_be_bytes(e, &1i8.to_be_bytes());
    assert_eq!(bi.to_i8(e), 1);

    let bi = BigInt::from_be_bytes(e, &(-1i8).to_be_bytes());
    assert_eq!(bi.to_i8(e), -1);

    let bi = BigInt::from_be_bytes(e, &0i128.to_be_bytes());
    assert_eq!(bi.to_i8(e), 0);
    assert_eq!(bi.to_i16(e), 0);
    assert_eq!(bi.to_i32(e), 0);
    assert_eq!(bi.to_i64(e), 0);
    assert_eq!(bi.to_i128(e), 0);
  }
}
