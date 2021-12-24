use super::*;
use crate::{get_size::*, to_bytes::*};

/// Errors encounterable serializing to bytes
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum MessageToBytesError {
  /// Reserved capacity was not enough for size of message
  TooLong { capacity: usize, size: usize },
}

impl<const PAYLOAD_CAP: usize, const N_OPTS: usize, const OPT_CAP: usize> TryIntoBytes
  for Message<PAYLOAD_CAP, N_OPTS, OPT_CAP>
{
  type Error = MessageToBytesError;

  fn try_into_bytes<const CAP: usize>(self) -> Result<ArrayVec<[u8; CAP]>, Self::Error> {
    let size: usize = self.get_size();
    if CAP < size {
      Err(Self::Error::TooLong { capacity: CAP, size })?
    }

    let mut bytes = ArrayVec::<[u8; CAP]>::new();

    let byte1: u8 = Byte1 { tkl: self.tkl,
                            ver: self.ver,
                            ty: self.ty }.into();
    let code: u8 = self.code.into();
    let id: [u8; 2] = self.id.into();
    let token: ArrayVec<[u8; 8]> = self.token.into();

    bytes.push(byte1);
    bytes.push(code);

    bytes.extend(id);
    bytes.extend(token);

    for opt in self.opts.into_iter() {
      opt.extend_bytes(&mut bytes);
    }

    if self.payload.0.len() > 0 {
      bytes.push(0b11111111);
      bytes.extend(self.payload.0);
    }

    Ok(bytes)
  }
}

pub(crate) fn opt_len_or_delta(val: u16) -> (u8, Option<ArrayVec<[u8; 2]>>) {
  match val {
    | n if n >= 269 => {
      let mut bytes = ArrayVec::new();
      bytes.extend((n - 269).to_be_bytes());
      (14, Some(bytes))
    },
    | n if n >= 13 => {
      let mut bytes = ArrayVec::new();
      bytes.push((n as u8) - 13);
      (13, Some(bytes))
    },
    | n => (n as u8, None),
  }
}

impl Into<ArrayVec<[u8; 8]>> for Token {
  fn into(self) -> ArrayVec<[u8; 8]> {
    self.0.to_be_bytes().into_iter().filter(|&b| b != 0).collect()
  }
}

impl<const OPT_CAP: usize> Opt<OPT_CAP> {
  fn extend_bytes(self, bytes: &mut impl Extend<u8>) {
    let (del, del_bytes) = opt_len_or_delta(self.delta.0);
    let (len, len_bytes) = opt_len_or_delta(self.value.0.len() as u16);
    let del = del << 4;

    let header = del | len;

    bytes.extend(Some(header));

    if let Some(bs) = del_bytes {
      bytes.extend(bs);
    }

    if let Some(bs) = len_bytes {
      bytes.extend(bs);
    }

    bytes.extend(self.value.0);
  }
}

impl<const OPT_CAP: usize> TryIntoBytes for Opt<OPT_CAP> {
  type Error = MessageToBytesError;

  fn try_into_bytes<const CAP: usize>(self) -> Result<ArrayVec<[u8; CAP]>, Self::Error> {
    let mut bytes = ArrayVec::<[u8; CAP]>::new();
    self.extend_bytes(&mut bytes);
    Ok(bytes)
  }
}

impl Into<u8> for Byte1 {
  fn into(self) -> u8 {
    let ver = self.ver.0 << 6;
    let ty = self.ty.0 << 4;
    let tkl = self.tkl.0;

    ver | ty | tkl
  }
}

impl Into<u8> for Code {
  fn into(self) -> u8 {
    let class = self.class << 5;
    let detail = self.detail;

    class | detail
  }
}

impl Into<[u8; 2]> for Id {
  fn into(self) -> [u8; 2] {
    self.0.to_be_bytes()
  }
}

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
    assert_eq!(actual, MessageToBytesError::TooLong { capacity: 8, size: 21 });
  }

  #[test]
  fn token() {
    let token = Token(12);
    let expected = vec![12u8];
    let actual: ArrayVec<u8, 8> = token.into();
    assert_eqb_iter!(actual, expected);

    let token = Token(0b11110000_11110000_11110000_11110000_11110000_11110000_11110000_11110000);
    let expected = core::iter::repeat(0b11110000u8).take(8).collect::<Vec<_>>();
    let actual: ArrayVec<u8, 8> = token.into();
    assert_eqb_iter!(actual, expected);
  }

  #[test]
  fn byte_1() {
    let byte = Byte1 { ver: Version(1),
                       ty: Type(2),
                       tkl: TokenLength(3) };
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
}
