//! JNI abstractions and bindings used by the toad ecosystem
//!
//! ## class bindings
//!
//! ## class shims
//! the `cls` module

// docs
#![doc(html_root_url = "https://docs.rs/toad-jni/0.0.0")]
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
