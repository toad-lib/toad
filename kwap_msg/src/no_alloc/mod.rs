use tinyvec::ArrayVec;

pub use crate::{from_bytes::*, GetSize, TryIntoBytes};

pub(crate) mod impl_from_bytes;
pub(crate) mod impl_get_size;
pub(crate) mod impl_to_bytes;

#[doc(hidden)]
pub mod opt;

#[doc(inline)]
pub use opt::*;

#[doc = include_str!("../../docs/no_alloc/Message.md")]
#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub struct Message<const PAYLOAD_CAP: usize, const N_OPTS: usize, const OPT_CAP: usize> {
  /// see [`Id`] for details
  pub id: Id,
  /// see [`Type`] for details
  pub ty: Type,
  /// see [`Version`] for details
  pub ver: Version,
  /// see [`Token`] for details
  pub token: Token,
  /// see [`Code`] for details
  pub code: Code,
  /// see [`opt::Opt`] for details
  pub opts: ArrayVec<[opt::Opt<OPT_CAP>; N_OPTS]>,
  /// See [`Payload`]
  pub payload: Payload<PAYLOAD_CAP>,
}







#[cfg(test)]
pub(self) fn test_msg() -> (Message<13, 1, 16>, Vec<u8>) {
  let header: [u8; 4] = 0b01_00_0001_01000101_0000000000000001u32.to_be_bytes();
  let token: [u8; 1] = [254u8];
  let content_format: &[u8] = b"application/json";
  let options: [&[u8]; 2] = [&[0b_1100_1101u8, 0b00000011u8], content_format];
  let payload: [&[u8]; 2] = [&[0b_11111111u8], b"hello, world!"];
  let bytes = [header.as_ref(),
               token.as_ref(),
               options.concat().as_ref(),
               payload.concat().as_ref()].concat();

  let mut opts = ArrayVec::new();
  let opt = Opt::<16> { delta: OptDelta(12),
                        value: OptValue(content_format.iter().copied().collect()) };
  opts.push(opt);

  let msg = Message::<13, 1, 16> { id: Id(1),
                                   ty: Type(0),
                                   ver: Version(1),
                                   token: Token(tinyvec::array_vec!([u8; 8] => 254)),
                                   opts,
                                   code: Code { class: 2, detail: 5 },
                                   payload: Payload(b"hello, world!".into_iter().copied().collect()) };
  (msg, bytes)
}
