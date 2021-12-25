use super::*;
use crate::is_full::IsFull;

impl<'a, const PAYLOAD_CAP: usize, const N_OPTS: usize, const OPT_CAP: usize> TryFromBytes<&'a u8>
  for Message<PAYLOAD_CAP, N_OPTS, OPT_CAP>
{
  type Error = MessageParseError;
  fn try_from_bytes<I: IntoIterator<Item = &'a u8>>(bytes: I) -> Result<Self, Self::Error> {
    Self::try_from_bytes(bytes.into_iter().copied())
  }
}

impl<const PAYLOAD_CAP: usize, const N_OPTS: usize, const OPT_CAP: usize> TryFromBytes<u8>
  for Message<PAYLOAD_CAP, N_OPTS, OPT_CAP>
{
  type Error = MessageParseError;

  fn try_from_bytes<I: IntoIterator<Item = u8>>(bytes: I) -> Result<Self, Self::Error> {
    let mut bytes = bytes.into_iter();

    let Byte1 { tkl, ty, ver } = Self::Error::try_next(&mut bytes)?.into();

    if tkl.0 > 8 {
      return Err(Self::Error::InvalidTokenLength(tkl.0 as u8));
    }

    let code: Code = Self::Error::try_next(&mut bytes)?.into();
    let id: Id = Id::try_consume_bytes(&mut bytes)?;
    let token = Token::try_consume_bytes(&mut bytes.by_ref().take(tkl.0 as usize))?;
    let opts = ArrayVec::<[Opt<OPT_CAP>; N_OPTS]>::try_consume_bytes(&mut bytes).map_err(Self::Error::OptParseError)?;
    let mut payload_bytes = ArrayVec::new();
    for byte in bytes {
      if let Some(_) = payload_bytes.try_push(byte) {
        return Err(Self::Error::PayloadTooLong(PAYLOAD_CAP));
      }
    }

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

impl<I: Iterator<Item = u8>> TryConsumeBytes<I> for Id {
  type Error = MessageParseError;
  fn try_consume_bytes(bytes: &mut I) -> Result<Self, Self::Error> {
    let taken_bytes = bytes.take(2).collect::<ArrayVec<[_; 2]>>();
    if taken_bytes.is_full() {
      Ok(taken_bytes.into_inner()).map(|bs| Id(u16::from_be_bytes(bs)))
    } else {
      Err(MessageParseError::UnexpectedEndOfStream)
    }
  }
}

impl<I: Iterator<Item = u8>> TryConsumeBytes<I> for Token {
  type Error = MessageParseError;

  fn try_consume_bytes(bytes: &mut I) -> Result<Self, Self::Error> {
    let bytes = bytes.into_iter().collect::<ArrayVec<[_; 8]>>();

    // pad the front with zeroes and copy values to array
    let bytes_u64 = core::iter::repeat(0u8).take(8 - bytes.len())
                                           .chain(bytes.into_iter())
                                           .collect::<ArrayVec<[u8; 8]>>()
                                           .into_inner();

    Ok(Token(u64::from_be_bytes(bytes_u64)))
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
    assert_eq!(Message::<13, 1, 16>::try_from_bytes(&msg).unwrap(), expect)
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
    let id = Id::try_consume_bytes(&mut id_bytes.iter().copied()).unwrap();
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
    let valid_a = Token::try_consume_bytes(&mut valid_a.iter().copied()).unwrap();
    assert_eq!(valid_a, Token(1));

    let valid_b: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 1];
    let valid_b = Token::try_consume_bytes(&mut valid_b.iter().copied()).unwrap();
    assert_eq!(valid_a, valid_b);
  }
}
