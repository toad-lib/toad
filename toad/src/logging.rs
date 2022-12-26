use core::fmt::Write;

use tinyvec::ArrayVec;
use toad_common::*;

use crate::platform;
use crate::todo::code_to_human;

pub(crate) fn msg_summary<P: platform::PlatformTypes>(msg: &platform::Message<P>)
                                                      -> Writable<ArrayVec<[u8; 64]>> {
  let mut buf: Writable<ArrayVec<[u8; 64]>> = Default::default();
  write!(buf,
         "{:?}: {:?} {} with {} byte payload",
         msg.code.kind(),
         msg.ty,
         code_to_human(msg.code).as_str(),
         msg.payload.0.get_size()).ok();
  buf
}
