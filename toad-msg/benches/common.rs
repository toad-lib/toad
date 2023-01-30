use tinyvec::ArrayVec;
use toad_msg::*;

pub type StackMessage<const PAYLOAD_BYTES: usize, const OPTS_MAX: usize, const OPT_BYTES: usize> = Message<ArrayVec<[u8; PAYLOAD_BYTES]>, ArrayVec<[(OptNumber, ArrayVec<[OptValue<ArrayVec<[u8; OPT_BYTES]>>; 1]>); OPTS_MAX]>>;
