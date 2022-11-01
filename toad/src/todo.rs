//! Future inherent methods on structs in other crates
use core::fmt::Write;
use core::ops::{Div, Mul};

use tinyvec::ArrayVec;
use toad_common::*;

/// A [`Map`](toad_common::Map) stored completely on the stack
pub type StackMap<K, V, const N: usize> = ArrayVec<[(K, V); N]>;

/// String with capacity of 1KB
#[derive(Debug, Copy, Clone, Default)]
pub struct String1Kb(Writable<ArrayVec<[u8; 1024]>>);

impl PartialEq for String1Kb {
  fn eq(&self, other: &Self) -> bool {
    self.0.as_str() == other.0.as_str()
  }
}

impl Eq for String1Kb {}

impl core::fmt::Write for String1Kb {
  fn write_str(&mut self, s: &str) -> core::fmt::Result {
    self.0.write_str(s)
  }
}

impl<'a> From<&'a str> for String1Kb {
  fn from(s: &'a str) -> Self {
    let mut arr = Writable::default();
    ArrayVec::extend_from_slice(&mut arr, s.as_bytes());

    Self(arr)
  }
}

impl AsRef<str> for String1Kb {
  fn as_ref(&self) -> &str {
    self.0.as_str()
  }
}

pub(crate) trait Capacity: GetSize {
  fn capacity(&self) -> Option<f32> {
    self.max_size()
        .map(|max| self.get_size() as f32 / max as f32)
  }

  fn capacity_pct(&self) -> Option<f32> {
    self.capacity().map(|dec| dec.mul(10000.).round().div(100.))
  }
}

impl<T: GetSize> Capacity for T {}

pub(crate) fn code_to_human(code: toad_msg::Code) -> Writable<ArrayVec<[u8; 4]>> {
  let mut buf: Writable<ArrayVec<[u8; 4]>> = Writable::default();
  code.to_human().iter().for_each(|char| {
                          write!(buf, "{}", char).ok();
                        });
  buf
}

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
    self.perform_err(|e| match e {
          | ::nb::Error::Other(e) => f(e),
          | ::nb::Error::WouldBlock => (),
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
