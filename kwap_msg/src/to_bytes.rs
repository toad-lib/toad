use tinyvec::ArrayVec;

use crate::*;

/// Trait allowing fallible conversion into bytes
pub trait TryIntoBytes {
  type Error;

  /// Try to convert into a collection of bytes
  ///
  /// ```
  /// use kwap_msg::TryIntoBytes;
  ///
  /// // This one has static params that allocates space on the static
  /// // and uses `tinyvec::ArrayVec` as the byte buffer backing structure
  /// let arrayvec_message = kwap_msg::ArrayVecMessage::<0, 0, 0> {
  ///   // ...
  /// # id: kwap_msg::Id(0),
  /// # ty: kwap_msg::Type(0),
  /// # ver: Default::default(),
  /// # opts: Default::default(),
  /// # payload: kwap_msg::Payload(Default::default()),
  /// # token: kwap_msg::Token(Default::default()),
  /// # code: kwap_msg::Code {class: 0, detail: 1},
  /// # __optc: Default::default(),
  /// };
  ///
  /// let bytes: tinyvec::ArrayVec<[u8; 1024]> = arrayvec_message.try_into_bytes().unwrap();
  ///
  /// // This one uses Vec
  /// let vec_message = kwap_msg::VecMessage {
  ///   // ...
  /// # id: kwap_msg::Id(0),
  /// # ty: kwap_msg::Type(0),
  /// # ver: Default::default(),
  /// # opts: Default::default(),
  /// # payload: kwap_msg::Payload(Default::default()),
  /// # token: kwap_msg::Token(Default::default()),
  /// # code: kwap_msg::Code {class: 0, detail: 1},
  /// # __optc: Default::default(),
  /// };
  ///
  /// let bytes: Vec<u8> = vec_message.try_into_bytes().unwrap();
  /// ```
  fn try_into_bytes<C: Collection<u8>>(self) -> Result<C, Self::Error>
    where for<'a> &'a C: IntoIterator<Item = &'a u8>;
}

/// Errors encounterable serializing to bytes
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum MessageToBytesError {
  /// Reserved capacity was not enough for size of message
  TooLong { capacity: usize, size: usize },
}

impl<P: Collection<u8>, O: Collection<u8>, Os: Collection<Opt<O>>> TryIntoBytes for Message<P, O, Os>
  where for<'b> &'b P: IntoIterator<Item = &'b u8>,
        for<'b> &'b O: IntoIterator<Item = &'b u8>,
        for<'b> &'b Os: IntoIterator<Item = &'b Opt<O>>
{
  type Error = MessageToBytesError;

  fn try_into_bytes<C: Collection<u8>>(self) -> Result<C, Self::Error>
    where for<'a> &'a C: IntoIterator<Item = &'a u8>
  {
    let mut bytes = C::reserve(1024);
    let size: usize = self.get_size();

    if let Some(max) = bytes.max_size() {
      if max < size {
        return Err(Self::Error::TooLong { capacity: max, size });
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

    for opt in self.opts.into_iter() {
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

impl Into<[u8; 2]> for Id {
  fn into(self) -> [u8; 2] {
    self.0.to_be_bytes()
  }
}

impl Into<u8> for Byte1 {
  fn into(self) -> u8 {
    let ver = self.ver.0 << 6;
    let ty = self.ty.0 << 4;
    let tkl = self.tkl;

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
             $actual.into_iter().map(|b| format!("{:08b}", b)).collect::<Vec<_>>(),
             $expected.into_iter().map(|b| format!("{:08b}", b)).collect::<Vec<_>>())
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
                     let opt = Opt::<Vec<u8>> { delta: OptDelta(delta),
                                                value: OptValue(values.into_iter().collect()) };
                     let mut actual = Vec::<u8>::new();
                     opt.extend_bytes(&mut actual);
                     assert_eqb_iter!(actual, expected)
                   });
}

#[test]
fn no_payload_marker() {
  let msg = VecMessage { id: Id(0),
                         ty: Type(0),
                         ver: Default::default(),
                         code: Code { class: 2, detail: 5 },
                         token: Token(Default::default()),
                         opts: Default::default(),
                         payload: Payload(Default::default()),
                         __optc: Default::default() };

  assert_ne!(msg.try_into_bytes::<Vec<_>>().unwrap().last(), Some(&0b11111111));
}
}
