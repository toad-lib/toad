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
