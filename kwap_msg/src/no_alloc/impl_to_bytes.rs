use super::*;
use crate::{get_size::*, to_bytes::*};


#[cfg(test)]
mod tests {
  use super::*;
  use crate::to_bytes::TryIntoBytes;

  macro_rules! assert_eqb {
    ($actual:expr, $expected:expr) => {
      if $actual != $expected {
        panic!("expected {:08b} to equal {:08b}", $actual, $expected)
      }
    };
  }
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
    let actual = m.try_into_bytes::<512>().unwrap().into_iter().collect::<Vec<_>>();
    assert_eqb_iter!(actual, expected);

    // shouldn't panic when message larger than capacity
    let (m, _) = super::super::test_msg();
    let actual = m.try_into_bytes::<8>().unwrap_err();
    assert_eq!(actual, MessageToBytesError::TooLong { capacity: 8, size: 37 });
  }

  #[test]
  fn byte_1() {
    let byte = Byte1 { ver: Version(1),
                       ty: Type(2),
                       tkl: 3 };
    let actual: u8 = byte.into();
    let expected = 0b_01_10_0011u8;
    assert_eqb!(actual, expected)
  }

  #[test]
  fn code() {
    let code = Code { class: 2, detail: 5 };
    let actual: u8 = code.into();
    let expected = 0b_010_00101u8;
    assert_eqb!(actual, expected)
  }

  #[test]
  fn id() {
    let id = Id(16);
    let actual = u16::from_be_bytes(id.into());
    assert_eqb!(actual, 16)
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
                       let opt = Opt::<300> { delta: OptDelta(delta),
                                              value: OptValue(values.into_iter().collect()) };
                       let actual = opt.try_into_bytes::<400>().unwrap();
                       assert_eqb_iter!(actual, expected)
                     });
  }

  #[test]
  fn no_payload_marker() {
    let msg = Message::<0, 0, 0> { id: Id(0),
                                   ty: Type(0),
                                   ver: Default::default(),
                                   code: Code { class: 2, detail: 5 },
                                   token: Token(Default::default()),
                                   opts: Default::default(),
                                   payload: Payload(Default::default()) };

    assert_ne!(msg.try_into_bytes::<20>().unwrap().last(), Some(&0b11111111));
  }
}
