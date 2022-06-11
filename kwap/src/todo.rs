//! Future inherent methods on structs in other crates
use core::fmt::Write;
use core::ops::{Div, Mul};

use kwap_common::prelude::*;
use tinyvec::ArrayVec;

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

pub(crate) fn code_to_human(code: kwap_msg::Code) -> Writable<ArrayVec<[u8; 4]>> {
  let mut buf: Writable<ArrayVec<[u8; 4]>> = Writable::default();
  code.to_human().iter().for_each(|char| {
                          write!(buf, "{}", char).ok();
                        });
  buf
}

#[derive(Debug)]
#[must_use]
pub(crate) struct Should<T, E>(Result<T, E>);

impl<T, E> Should<T, E> {
  pub fn should_pass(self, f: impl FnOnce(&T) -> bool) -> Else<T, E> {
    Else(self.0.map(|t| (f(&t), t)))
  }
}

impl<T: PartialEq, E> Should<T, E> {
  pub fn should_eq(self, other: &T) -> Else<T, E> {
    self.should_pass(|t| t == other)
  }
}

#[derive(Debug)]
#[must_use]
pub(crate) struct Else<T, E>(Result<(bool, T), E>);
impl<T, E> Else<T, E> {
  pub fn else_err(self, f: impl FnOnce(T) -> E) -> Result<T, E> {
    self.0.bind(|(pass, t)| match pass {
            | false => Err(f(t)),
            | true => Ok(t),
          })
  }
}

pub(crate) trait ResultExt2<T, E> {
  fn validate(self, f: impl FnOnce(Should<T, E>) -> Result<T, E>) -> Result<T, E>;
}

impl<T, E> ResultExt2<T, E> for Result<T, E> {
  fn validate(self, f: impl FnOnce(Should<T, E>) -> Result<T, E>) -> Result<T, E> {
    f(Should(self))
  }
}
