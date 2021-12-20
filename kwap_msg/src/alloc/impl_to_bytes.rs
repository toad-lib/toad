use arrayvec::ArrayVec;
use std_alloc::vec::Vec;

use super::*;
use crate::no_alloc::impl_to_bytes::opt_len_or_delta;

// TODO(orion): Shame about all this duplicated code :thinking:

impl Into<Vec<u8>> for Message {
  fn into(self) -> Vec<u8> {
    let byte1: u8 = Byte1 { tkl: self.tkl,
                            ver: self.ver,
                            ty: self.ty }.into();
    let code: u8 = self.code.into();
    let id: [u8; 2] = self.id.into();
    let token: ArrayVec<u8, 8> = self.token.into();
    let opts: Vec<u8> = self.opts
                            .into_iter()
                            .map(|o| -> Vec<u8> { o.into() })
                            .flatten()
                            .collect();

    let bytes: Vec<u8> = core::iter::once(byte1).chain(core::iter::once(code))
                                                .chain(id)
                                                .chain(token)
                                                .chain(opts)
                                                .chain(core::iter::once(0b11111111))
                                                .chain(self.payload.0.iter().copied())
                                                .collect();

    bytes
  }
}

impl Into<Vec<u8>> for Opt {
  fn into(self) -> Vec<u8> {
    let (del, del_bytes) = opt_len_or_delta(self.delta.0);
    let (len, len_bytes) = opt_len_or_delta(self.value.0.len() as u16);
    let del = del << 4;

    let header = del | len;

    let bytes = core::iter::once(header).chain(del_bytes.unwrap_or_default())
                                        .chain(len_bytes.unwrap_or_default())
                                        .chain(self.value.0)
                                        .collect();

    bytes
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  macro_rules! assert_eqb_iter {
    ($actual:expr, $expected:expr) => {
      if $actual.iter().ne($expected.iter()) {
        panic!("expected {:?} to equal {:?}",
               $actual.into_iter().map(|b| format!("{:08b}", b)).collect::<Vec<_>>(),
               $expected.into_iter().map(|b| format!("{:08b}", b)).collect::<Vec<_>>())
      }
    };
  }

  #[test]
  fn msg() {
    let (m, expected) = super::super::test_msg();
    let actual: Vec<u8> = m.into();
    assert_eqb_iter!(actual, expected);
  }

  #[test]
  fn opt() {
    use core::iter::repeat;
    let cases: [(u16, Vec<u8>, Vec<u8>); 4] = [(24,
                                                repeat(1).take(100).collect(),
                                                [[0b1101_1101u8, 24 - 13, 100 - 13].as_ref(),
                                                 repeat(1).take(100).collect::<Vec<u8>>().as_ref()].concat()),
                                               (1, vec![1], vec![0b0001_0001, 1]),
                                               (24, vec![1], vec![0b1101_0001, 11, 1]),
                                               (24,
                                                repeat(1).take(300).collect(),
                                                [[0b1101_1110, 24 - 13].as_ref(),
                                                 (300u16 - 269).to_be_bytes().as_ref(),
                                                 repeat(1).take(300).collect::<Vec<u8>>().as_ref()].concat())];

    cases.into_iter().for_each(|(delta, values, expected)| {
                       let opt = Opt { delta: OptDelta(delta),
                                       value: OptValue(values.into_iter().collect()) };
                       let actual: Vec<u8> = opt.into();
                       assert_eqb_iter!(actual, expected)
                     });
  }
}
