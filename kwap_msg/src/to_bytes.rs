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
  fn try_into_bytes<C: Collection<u8>>(self) -> Result<C, Self::Error> where for<'a> &'a C: IntoIterator<Item = &'a u8>;
}

/// Errors encounterable serializing to bytes
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum MessageToBytesError {
  /// Reserved capacity was not enough for size of message
  TooLong { capacity: usize, size: usize },
}

impl<P: Collection<u8>, O: Collection<u8>, Os: Collection<Opt<O>>> TryIntoBytes for Message<P, O, Os>
where
    for<'b> &'b P: IntoIterator<Item = &'b u8>,
    for<'b> &'b O: IntoIterator<Item = &'b u8>,
    for<'b> &'b Os: IntoIterator<Item = &'b Opt<O>>,
    {
  type Error = MessageToBytesError;

  fn try_into_bytes<C: Collection<u8>>(self) -> Result<C, Self::Error> where for<'a> &'a C: IntoIterator<Item = &'a u8>{
    let mut bytes = C::default();
    let size: usize = self.get_size();
    if bytes.capacity() < size {
      return Err(Self::Error::TooLong { capacity: bytes.capacity(),
                                        size });
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

    if !self.payload.0.is_empty() {
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

impl<C: Collection<u8>> Opt<C>  where for<'b> &'b C: IntoIterator<Item = &'b u8>{
  fn extend_bytes(self, bytes: &mut impl Extend<u8>) {
    let (del, del_bytes) = opt_len_or_delta(self.delta.0);
    let (len, len_bytes) = opt_len_or_delta(self.value.0.get_size() as u16);
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
