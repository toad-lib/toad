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
#![cfg_attr(not(test), no_std)]

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
  use toad_jni::convert::Primitive;
  use toad_jni::{ArrayList, Sig};

  pub use crate as toad_jni;

  #[test]
  fn sig() {
    assert_eq!(Sig::new().returning(Sig::VOID).as_str(), "()V");
    assert_eq!(Sig::new().arg(Sig::array_of(Sig::BYTE))
                         .returning(Sig::class("java/lang/Byte"))
                         .as_str(),
               "([B)Ljava/lang/Byte;");
  }

  #[test]
  fn tests() {
    toad_jni::global::init();

    assert_eq!(i8::dewrap((32i8).wrap()), 32i8);
    assert_eq!(vec![1i8, 2, 3, 4].into_iter()
                                 .collect::<ArrayList<i8>>()
                                 .into_iter()
                                 .collect::<Vec<i8>>(),
               vec![1, 2, 3, 4])
  }
}
