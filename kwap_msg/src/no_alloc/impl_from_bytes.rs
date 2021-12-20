use super::*;

impl<const PAYLOAD_CAP: usize, const N_OPTS: usize, const OPT_CAP: usize> TryFromBytes
  for Message<PAYLOAD_CAP, N_OPTS, OPT_CAP>
{
  type Error = MessageParseError;

  fn try_from_bytes<T: IntoIterator<Item = u8>>(bytes: T) -> Result<Self, Self::Error> {
    let mut bytes = bytes.into_iter();

    let Byte1 { tkl, ty, ver } = Self::Error::try_next(&mut bytes)?.into();

    if tkl.0 > 8 {
      Err(Self::Error::InvalidTokenLength(tkl.0 as u8))?;
    }

    let code: Code = Self::Error::try_next(&mut bytes)?.into();
    let id: Id = Id::try_consume_bytes(&mut bytes)?;
    let token = Token::try_consume_bytes(bytes.by_ref().take(tkl.0 as usize))?;
    let opts = ArrayVec::<Opt<OPT_CAP>, N_OPTS>::try_consume_bytes(&mut bytes).map_err(Self::Error::OptParseError)?;
    let mut payload_bytes = ArrayVec::new();
    bytes.try_for_each(|b| {
           payload_bytes.try_push(b)
                        .map_err(|_| Self::Error::PayloadTooLong(PAYLOAD_CAP))
         })?;

    let payload = Payload(payload_bytes);

    Ok(Message { tkl,
                 id,
                 ty,
                 ver,
                 code,
                 token,
                 opts,
                 payload })
  }
}
impl From<u8> for Byte1 {
  fn from(b: u8) -> Self {
    let ver = b >> 6; // bits 0 & 1
    let ty = b >> 4 & 0b11; // bits 2 & 3
    let tkl = b & 0b1111u8; // last 4 bits

    Byte1 { ver: Version(ver),
            ty: Type(ty),
            tkl: TokenLength(tkl) }
  }
}

impl<T: IntoIterator<Item = u8>> TryConsumeBytes<T> for Id {
  type Error = MessageParseError;
  fn try_consume_bytes(bytes: T) -> Result<Self, Self::Error> {
    let bytes = bytes.into_iter().take(2).collect::<ArrayVec<_, 2>>();
    bytes.into_inner()
         .map(|bs| u16::from_be_bytes(bs))
         .map(Id)
         .map_err(|_| MessageParseError::UnexpectedEndOfStream)
  }
}

/// # PANICS
/// Panics when iterator passed to this implementation contains > 8 bytes.
impl<T: IntoIterator<Item = u8>> TryConsumeBytes<T> for Token {
  type Error = MessageParseError;

  fn try_consume_bytes(bytes: T) -> Result<Self, Self::Error> {
    let bytes = bytes.into_iter().collect::<ArrayVec<_, 8>>();

    let mut array_u64: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 0];

    // pad the front with zeroes and copy values to array
    core::iter::repeat(0u8).take(8 - bytes.len())
                           .chain(bytes.into_iter())
                           .enumerate()
                           .for_each(|(ix, b)| {
                             array_u64[ix] = b;
                           });

    Ok(Token(u64::from_be_bytes(array_u64)))
  }
}

impl From<u8> for Code {
  fn from(b: u8) -> Self {
    let class = b >> 5;
    let detail = b & 0b0011111;

    Code { class, detail }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_msg() {
    let (expect, msg) = super::super::test_msg();
    assert_eq!(Message::<13, 1, 16>::try_from_bytes(msg).unwrap(), expect)
  }

  #[test]
  fn parse_byte1() {
    let byte = 0b_01_10_0011u8;
    let byte = Byte1::from(byte);
    assert_eq!(byte,
               Byte1 { ver: Version(1),
                       ty: Type(2),
                       tkl: TokenLength(3) })
  }

  #[test]
  fn parse_id() {
    let id_bytes = 34u16.to_be_bytes();
    let id = Id::try_consume_bytes(id_bytes).unwrap();
    assert_eq!(id, Id(34));
  }

  #[test]
  fn parse_code() {
    let byte = 0b_01_000101u8;
    let code = Code::from(byte);
    assert_eq!(code, Code { class: 2, detail: 5 })
  }

  #[test]
  fn parse_token() {
    let valid_a: [u8; 1] = [0b_00000001u8];
    let valid_a = Token::try_consume_bytes(valid_a).unwrap();
    assert_eq!(valid_a, Token(1));

    let valid_b: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 1];
    let valid_b = Token::try_consume_bytes(valid_b).unwrap();
    assert_eq!(valid_a, valid_b);
  }
}
