use tinyvec::ArrayVec;
use toad_common::GetSize;

use crate::*;

/// Trait allowing fallible conversion into bytes
pub trait TryIntoBytes {
  type Error;

  /// Try to convert into a collection of bytes
  ///
  /// ```
  /// use tinyvec::ArrayVec;
  /// use toad_msg::{Message, OptNumber, OptValue, TryIntoBytes};
  ///
  /// type OptionValue = OptValue<ArrayVec<[u8; 128]>>;
  /// type OptionMapEntry = (OptNumber, ArrayVec<[OptionValue; 4]>);
  /// type OptionMap = ArrayVec<[OptionMapEntry; 16]>;
  /// type Payload = ArrayVec<[u8; 1024]>;
  /// let arrayvec_message = Message::<Payload, OptionMap> {
  ///   // ...
  /// # id: toad_msg::Id(0),
  /// # ty: toad_msg::Type::Con,
  /// # ver: Default::default(),
  /// # opts: Default::default(),
  /// # payload: toad_msg::Payload(Default::default()),
  /// # token: toad_msg::Token(Default::default()),
  /// # code: toad_msg::Code {class: 0, detail: 1},
  /// };
  ///
  /// let bytes: tinyvec::ArrayVec<[u8; 1024]> = arrayvec_message.try_into_bytes().unwrap();
  ///
  /// // This one uses Vec
  /// let vec_message = toad_msg::alloc::Message {
  ///   // ...
  /// # id: toad_msg::Id(0),
  /// # ty: toad_msg::Type::Con,
  /// # ver: Default::default(),
  /// # opts: Default::default(),
  /// # payload: toad_msg::Payload(Default::default()),
  /// # token: toad_msg::Token(Default::default()),
  /// # code: toad_msg::Code {class: 0, detail: 1},
  /// };
  ///
  /// let bytes: Vec<u8> = vec_message.try_into_bytes().unwrap();
  /// ```
  fn try_into_bytes<C: Array<Item = u8>>(self) -> Result<C, Self::Error>;
}

/// Errors encounterable serializing to bytes
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessageToBytesError {
  /// Reserved capacity was not enough for size of message
  TooLong { capacity: usize, size: usize },
}

impl<PayloadBytes: Array<Item = u8>, Options: OptionMap> TryIntoBytes
  for Message<PayloadBytes, Options>
{
  type Error = MessageToBytesError;

  fn try_into_bytes<C: Array<Item = u8>>(self) -> Result<C, Self::Error> {
    let mut bytes = C::reserve(self.get_size());
    let size: usize = self.get_size();

    if let Some(max) = bytes.max_size() {
      if max < size {
        return Err(Self::Error::TooLong { capacity: max,
                                          size });
      }
    }

    let byte1: u8 = Byte1 { tkl: self.token.0.len() as u8,
                            ver: self.ver,
                            ty: self.ty }.into();
    let code: u8 = self.code.into();
    let id: [u8; 2] = self.id.into();
    let token: ArrayVec<[u8; 8]> = self.token.0;

    bytes.extend(Some(byte1));
    bytes.extend(Some(code));

    bytes.extend(id);
    bytes.extend(token);

    for opt in self.opts.opts() {
      opt.extend_bytes(&mut bytes);
    }

    if !self.payload.0.size_is_zero() {
      bytes.extend(Some(0b11111111));
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

impl From<Id> for [u8; 2] {
  fn from(id: Id) -> [u8; 2] {
    id.0.to_be_bytes()
  }
}

impl From<Type> for u8 {
  fn from(t: Type) -> u8 {
    use Type::*;
    match t {
      | Con => 0,
      | Non => 1,
      | Ack => 2,
      | Reset => 3,
    }
  }
}

impl From<Byte1> for u8 {
  fn from(b: Byte1) -> u8 {
    let ver = b.ver.0 << 6;
    let ty = u8::from(b.ty) << 4;
    let tkl = b.tkl;

    ver | ty | tkl
  }
}

#[cfg(test)]
mod tests {
  use super::*;

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
               $actual.into_iter()
                      .map(|b| format!("{:08b}", b))
                      .collect::<Vec<_>>(),
               $expected.into_iter()
                        .map(|b| format!("{:08b}", b))
                        .collect::<Vec<_>>())
      }
    };
  }

  #[test]
  fn msg() {
    let (msg, expected) = test_msg();
    let actual: Vec<u8> = msg.try_into_bytes().unwrap();
    assert_eqb_iter!(actual, expected);
  }

  #[test]
  fn byte_1() {
    let byte = Byte1 { ver: Version(1),
                       ty: Type::Ack,
                       tkl: 3 };
    let actual: u8 = byte.into();
    let expected = 0b_01_10_0011u8;
    assert_eqb!(actual, expected)
  }

  #[test]
  fn code() {
    let code = Code { class: 2,
                      detail: 5 };
    let actual: u8 = code.into();
    let expected = 0b0100_0101_u8;
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
    let cases: [(u16, Vec<u8>, Vec<u8>); 4] =
      [(24,
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
                       let opt = Opt::<Vec<u8>> { delta: OptDelta(delta),
                                                  value: OptValue(values.into_iter().collect()) };
                       let mut actual = Vec::<u8>::new();
                       opt.extend_bytes(&mut actual);
                       assert_eqb_iter!(actual, expected)
                     });
  }

  #[test]
  fn no_payload_marker() {
    let msg = alloc::Message { id: Id(0),
                               ty: Type::Con,
                               ver: Default::default(),
                               code: Code { class: 2,
                                            detail: 5 },
                               token: Token(Default::default()),
                               opts: Default::default(),
                               payload: Payload(Default::default()) };

    assert_ne!(msg.try_into_bytes::<Vec<_>>().unwrap().last(),
               Some(&0b11111111));
  }
}
