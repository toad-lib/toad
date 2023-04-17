//! Future inherent methods on structs in other crates
use core::{ops::{Div, Mul}, fmt::Write};

use naan::prelude::ResultExt;
use tinyvec::ArrayVec;
use toad_len::Len;
use toad_writable::Writable;

pub mod hkt {
  pub trait Array {
    type Of<T: Default>: toad_array::Array<Item = T>;
  }

  #[cfg(feature = "alloc")]
  #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
  pub struct Vec;

  #[cfg(feature = "alloc")]
  impl Array for Vec {
    type Of<T: Default> = ::std_alloc::vec::Vec<T>;
  }

  #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
  pub struct ArrayVec<const N: usize>;
  impl<const N: usize> Array for ArrayVec<N> {
    type Of<T: Default> = tinyvec::ArrayVec<[T; N]>;
  }
}

/// A [`Map`](toad_common::Map) stored completely on the stack
pub type StackMap<K, V, const N: usize> = ArrayVec<[(K, V); N]>;

/// Stack-allocated mutable string with capacity of 1KB
#[derive(Debug, Copy, Clone, Default)]
pub struct String<const N: usize>(Writable<ArrayVec<[u8; N]>>);

impl<const N: usize> String<N> {
  /// Alias for [`AsRef`]
  pub fn as_str(&self) -> &str {
    self.as_ref()
  }

  pub fn fmt(args: core::fmt::Arguments) -> Self {
    let mut s = Self::default();
    s.write_fmt(args).ok();
    s
  }

  /// Resize the String to a new length
  ///
  /// If `M` is less than `N`, the extra bytes are
  /// discarded.
  pub fn resize<const M: usize>(&mut self) -> String<M> {
    let mut bytes = self.0.unwrap();
    bytes.truncate(M);
    String(Writable::from(self.as_writable().drain(..).collect::<ArrayVec<[u8; M]>>()))
  }

  /// Alias for [`AsRef`]
  pub fn as_bytes(&self) -> &[u8] {
    self.as_ref()
  }

  /// Get a mutable reference to the inner writable buffer
  pub fn as_writable(&mut self) -> &mut Writable<ArrayVec<[u8; N]>> {
    &mut self.0
  }
}

impl<const N: usize> PartialEq for String<N> {
  fn eq(&self, other: &Self) -> bool {
    self.0.as_str() == other.0.as_str()
  }
}

impl<const N: usize> Eq for String<N> {}

impl<const N: usize> core::fmt::Write for String<N> {
  fn write_str(&mut self, s: &str) -> core::fmt::Result {
    self.0.write_str(s)
  }
}

impl<'a, const N: usize> From<&'a str> for String<N> {
  fn from(s: &'a str) -> Self {
    let mut arr = Writable::default();
    ArrayVec::extend_from_slice(&mut arr, s.as_bytes());

    Self(arr)
  }
}

impl<const N: usize> AsRef<str> for String<N> {
  fn as_ref(&self) -> &str {
    self.0.as_str()
  }
}

impl<const N: usize> AsRef<[u8]> for String<N> {
  fn as_ref(&self) -> &[u8] {
    self.0.as_str().as_bytes()
  }
}

pub(crate) trait Capacity: Len {
  fn capacity(&self) -> Option<f32> {
    Self::CAPACITY.map(|max| self.len() as f32 / max as f32)
  }

  fn capacity_pct(&self) -> Option<f32> {
    self.capacity().map(|dec| dec.mul(10000.).round().div(100.))
  }
}

impl<T: Len> Capacity for T {}

pub(crate) trait ResultExt2<T, E> {
  fn unwrap_err_or(self, f: impl FnOnce(T) -> E) -> E;
  fn try_perform_mut(self, f: impl FnOnce(&mut T) -> Result<(), E>) -> Result<T, E>;
}

impl<T, E> ResultExt2<T, E> for Result<T, E> {
  fn unwrap_err_or(self, f: impl FnOnce(T) -> E) -> E {
    match self {
      | Ok(t) => f(t),
      | Err(e) => e,
    }
  }

  fn try_perform_mut(self, f: impl FnOnce(&mut T) -> Result<(), E>) -> Result<T, E> {
    match self {
      | Ok(mut t) => f(&mut t).map(|_| t),
      | Err(e) => Err(e),
    }
  }
}

pub(crate) trait NbResultExt<T, E> {
  fn perform_nb_err(self, f: impl FnOnce(&E) -> ()) -> Self;
  #[cfg(feature = "std")]
  fn expect_nonblocking(self, msg: impl ToString) -> Result<T, E>;
}

impl<T, E> NbResultExt<T, E> for ::nb::Result<T, E> {
  fn perform_nb_err(self, f: impl FnOnce(&E) -> ()) -> ::nb::Result<T, E> {
    self.discard_err(|e: &::nb::Error<E>| match e {
          | &::nb::Error::Other(ref e) => f(e),
          | &::nb::Error::WouldBlock => (),
        })
  }

  #[cfg(feature = "std")]
  fn expect_nonblocking(self, msg: impl ToString) -> Result<T, E> {
    match self {
      | Ok(ok) => Ok(ok),
      | Err(::nb::Error::Other(e)) => Err(e),
      | Err(::nb::Error::WouldBlock) => panic!("{}", msg.to_string()),
    }
  }
}

pub(crate) mod nb {
  #[allow(unused_macros)]
  macro_rules! nb_block {
    ($stuff:expr, with = $with:expr) => {
      loop {
        match $stuff {
          | Ok(t) => break Ok(t),
          | Err(::nb::Error::Other(e)) => break Err(e),
          | Err(::nb::Error::WouldBlock) => match $with() {
            | Some(ripcord) => break ripcord,
            | None => continue,
          },
        }
      }
    };
    ($stuff:expr, timeout_after = $duration:expr, timeout_err = $timeout_err:expr) => {{
      let start = ::std::time::Instant::now();
      $crate::todo::nb::block!($stuff,
                               with = || {
                                 if ::std::time::Instant::now() - start > $duration {
                                   Some(Err($timeout_err()))
                                 } else {
                                   None
                                 }
                               })
    }};
    ($stuff:expr, io_timeout_after = $duration:expr) => {
      $crate::todo::nb::block!($stuff,
                               timeout_after = $duration,
                               timeout_err =
                                 || ::std::io::Error::from(::std::io::ErrorKind::TimedOut))
    };
  }

  #[allow(unused_imports)]
  pub(crate) use nb_block as block;
}
