//! JNI abstractions and bindings used by the toad ecosystem

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

pub mod convert;
pub mod sig;

pub use cls::java::util::ArrayList;
pub use sig::Sig;

/// java class shims
pub mod cls;

/// Global JVM handles
pub mod global {
  use jni::{InitArgsBuilder, JNIEnv, JavaVM};

  static mut JVM: Option<JavaVM> = None;

  /// Initialize the global jvm handle with an existing handle
  pub fn init_with(jvm: JavaVM) {
    unsafe {
      JVM = Some(jvm);
    }
  }

  /// Initialize the global jvm handle by creating a new handle
  pub fn init() {
    let args = InitArgsBuilder::new().build().unwrap();
    unsafe {
      JVM = Some(JavaVM::new(args).unwrap());
    }

    jvm().attach_current_thread_permanently().unwrap();
  }

  /// Get a reference to the global jvm handle
  pub fn jvm() -> &'static mut JavaVM {
    unsafe { JVM.as_mut().unwrap() }
  }

  /// Create a new local frame from the global jvm handle
  pub fn env<'a>() -> JNIEnv<'a> {
    unsafe { JVM.as_mut().unwrap().get_env().unwrap() }
  }
}

#[cfg(test)]
mod test {
  use jni::objects::{JString, JThrowable};
  use jni::strings::JNIStr;
  use toad_jni::cls::java;
  use toad_jni::convert::Primitive;
  use toad_jni::{ArrayList, Sig};

  pub use crate as toad_jni;
  use crate::convert::Object;

  #[test]
  fn sig() {
    assert_eq!(Sig::new().returning(Sig::VOID).as_str(), "()V");
    assert_eq!(Sig::new().arg(Sig::array_of(Sig::BYTE))
                         .returning(Sig::class("java/lang/Byte"))
                         .as_str(),
               "([B)Ljava/lang/Byte;");
  }

  fn test_prim_wrappers() {
    assert_eq!(i8::dewrap((32i8).wrap()), 32i8);
  }

  fn test_arraylist() {
    assert_eq!(vec![1i8, 2, 3, 4].into_iter()
                                 .collect::<ArrayList<i8>>()
                                 .into_iter()
                                 .collect::<Vec<i8>>(),
               vec![1, 2, 3, 4])
  }

  fn test_optional() {
    let mut e = toad_jni::global::env();

    let o = java::util::Optional::of(&mut e, 12i32);
    assert_eq!(o.to_option(&mut e).unwrap(), 12);

    let o = java::util::Optional::<i32>::empty(&mut e);
    assert!(o.is_empty(&mut e));
  }

  fn test_time() {
    let mut e = toad_jni::global::env();

    let o = java::time::Duration::of_millis(&mut e, 1000);
    assert_eq!(o.to_millis(&mut e), 1000);
  }

  fn test_bigint() {
    let mut e = toad_jni::global::env();
    let e = &mut e;

    type BigInt = java::math::BigInteger;

    let bi = BigInt::from_be_bytes(e, &i128::MAX.to_be_bytes());
    assert_eq!(bi.to_i128(e), i128::MAX);

    let bi = BigInt::from_be_bytes(e, &i8::MAX.to_be_bytes());
    assert_eq!(bi.to_i128(e), i8::MAX.into());
  }

  #[test]
  fn tests() {
    toad_jni::global::init();

    test_prim_wrappers();
    test_arraylist();
    test_optional();
    test_time();
    test_bigint();
  }
}
