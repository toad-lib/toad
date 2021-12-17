use super::*;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;
use arrayvec::ArrayVec;

/// Trait allowing fallible conversion from bytes
pub trait TryConsumeBytes<T>: Sized {
  /// Error type yielded if conversion fails
  type Error;

  /// Try to convert from some sequence of bytes `T`
  /// into `Self`
  fn try_consume_bytes(bytes: T) -> Result<Self, Self::Error>;
}

/// Errors encounterable while parsing a message from bytes
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub enum MessageParseError {
  /// Reached end of stream before parsing was finished
  UnexpectedEndOfStream,

  /// Token length was > 8
  InvalidTokenLength(u8),

  /// Option Delta was set to 15, which is invalid.
  OptionDeltaReservedValue(u8),

  /// Value Length was set to 15, which is invalid.
  ValueLengthReservedValue(u8),

  /// Not a true failure case; only means we tried to read the payload marker byte (0xFF)
  /// as an option header.
  OptionsExhausted,
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
struct Byte1 {ver: Version, ty: Type, tkl: TokenLength}
impl From<u8> for Byte1 {
  fn from(b: u8) -> Self {
    let ver = b >> 6;
    let ty = b >> 4 & 0b11u8;
    let tkl = b & 0b1111u8;

    Byte1 {ver: Version(ver), ty: Type(ty), tkl: TokenLength(tkl)}
  }
}

fn try_next<I>(iter: &mut impl Iterator<Item = I>) -> Result<I, MessageParseError> {
  iter.next().ok_or(MessageParseError::UnexpectedEndOfStream)
}

#[cfg(feature = "alloc")]
#[cfg_attr(any(feature = "docs", docsrs), doc(cfg(feature = "alloc")))]
impl<'a, T: IntoIterator<Item = &'a u8>> TryConsumeBytes<T> for Message {
  type Error = MessageParseError;

  fn try_consume_bytes(bytes: T) -> Result<Self, Self::Error> {
    let mut bytes = bytes.into_iter().map(|b| *b);

    let Byte1 {tkl, ty, ver} = try_next(&mut bytes)?.into();
    let code: Code = try_next(&mut bytes)?.into();
    let id: Id = Id::try_consume_bytes(&mut bytes)?;
    let token = Token::try_consume_bytes(bytes.by_ref().take(tkl.0 as usize))?;
    let opts = Vec::<opt::Opt>::try_consume_bytes(&mut bytes)?;
    let payload = Payload(bytes.collect());

    Ok(Message {tkl, id, ty, ver, code, token, opts, payload})
  }
}

impl From<u8> for Code {
  fn from(b: u8) -> Self {
    let class = b >> 5;
    let detail = b & 0b0011111;

    Code {class, detail}
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

impl<T: IntoIterator<Item = u8>> TryConsumeBytes<T> for Token {
  type Error = MessageParseError;

  fn try_consume_bytes(bytes: T) -> Result<Self, Self::Error> {
    let bytes = bytes.into_iter().collect::<Vec<_>>();
    let mut array_u64: [u8; 8] = [0,0,0,0,0,0,0,0];

    if bytes.len() > 8 {
      Err(MessageParseError::InvalidTokenLength(bytes.len() as u8))
    } else {
      // pad the front with zeroes
      core::iter::repeat(0u8)
          .take(8 - bytes.len())
          .chain(bytes.into_iter())
          .enumerate()
          .for_each(|(ix, b)| {
            array_u64[ix] = b;
          });

        Ok(Token(u64::from_be_bytes(array_u64)))
    }
  }
}

impl<T: IntoIterator<Item = u8>> TryConsumeBytes<T> for Vec<opt::Opt> {
  type Error = MessageParseError;

  fn try_consume_bytes(bytes: T) -> Result<Self, Self::Error> {
    let mut bytes = bytes.into_iter();
    let mut opts = Vec::new();

    loop {
        match opt::Opt::try_consume_bytes(bytes.by_ref()) {
          Ok(opt) => {
            opts.push(opt);
          },
          Err(MessageParseError::OptionsExhausted) => break Ok(opts),
          Err(e) => break Err(e),
        }
    }
  }
}

impl<T: IntoIterator<Item = u8>> TryConsumeBytes<T> for opt::Opt {
  type Error = MessageParseError;

  fn try_consume_bytes(bytes: T) -> Result<Self, Self::Error> {
    // delta will at most consume 3 bytes
    let mut bytes = bytes.into_iter().peekable();
    let opt_header = bytes.peek().copied().ok_or(MessageParseError::UnexpectedEndOfStream)?;

    if let 0b11111111 = opt_header {
      // This isn't an option, it's the payload!
      Err(MessageParseError::OptionsExhausted)?
    }

    // NOTE: Delta will consume at least the first byte of the iterator.
    //       If the Delta value is extended to 1 or 2 bytes, then it will consume those as well.
    //       By keeping a copy of the 1-byte header containing the non-extended delta and length,
    //       we can smoosh it on the front of `bytes` after being consumed by Delta.
    //
    //       This means that the Value consumer will see the header followed by the _Length_'s extended
    //       bytes, not the Delta's extended bytes since they were consumed already.
    let delta = opt::Delta::try_consume_bytes(&mut bytes)?;
    let value = opt::Value::try_consume_bytes(&mut [opt_header].into_iter().chain(bytes))?;
    Ok(opt::Opt{delta, value})
  }
}

fn interpret_opt_header(head: u8, bytes: &mut impl Iterator<Item = u8>) -> Result<u16, MessageParseError> {
    if head == 15 {
      Err(MessageParseError::OptionDeltaReservedValue(head))?
    }

    match head {
      13 => {
        let n = try_next(bytes)?;
        Ok((n as u16) + 13)
      },
      14 => {
        bytes
            .take(2)
            .collect::<ArrayVec<_, 2>>()
            .into_inner()
            .map(|array| u16::from_be_bytes(array) + 269)
            .map_err(|_| MessageParseError::UnexpectedEndOfStream)
      },
      _ => Ok(head as u16),
    }
}

impl<T: IntoIterator<Item = u8>> TryConsumeBytes<T> for opt::Delta {
  type Error = MessageParseError;

  fn try_consume_bytes(bytes: T) -> Result<Self, Self::Error> {
    let mut bytes = bytes.into_iter();
    let first_byte = try_next(&mut bytes)?;
    let delta = first_byte >> 4;
    let delta = interpret_opt_header(delta, &mut bytes)?;

    Ok(opt::Delta(delta))
  }
}

impl<T: IntoIterator<Item = u8>> TryConsumeBytes<T> for opt::Value {
  type Error = MessageParseError;

  fn try_consume_bytes(bytes: T) -> Result<Self, Self::Error> {
    let mut bytes = bytes.into_iter();
    let first_byte = try_next(&mut bytes)?;
    let len = first_byte & 0b00001111;
    let len = interpret_opt_header(len, &mut bytes)?;

    let data: Vec<u8> = bytes.take(len as usize).collect();
    if data.len() < len as usize {
      Err(MessageParseError::UnexpectedEndOfStream)
    } else {
      Ok(opt::Value {len, data})
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn byte1() {
    let byte = 0b_01_10_0011u8;
    let byte = Byte1::from(byte);
    assert_eq!(byte, Byte1 {ver: Version(1), ty: Type(2), tkl: TokenLength(3)})
  }

  #[test]
  fn code() {
    let byte = 0b_01_000101u8;
    let code = Code::from(byte);
    assert_eq!(code, Code {class: 2, detail: 5})
  }

  #[test]
  fn token() {
    let valid_a: [u8; 1] = [0b_00000001u8];
    let valid_a = Token::try_consume_bytes(valid_a).unwrap();
    assert_eq!(valid_a, Token(1));

    let valid_b: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 1];
    let valid_b = Token::try_consume_bytes(valid_b).unwrap();
    assert_eq!(valid_a, valid_b);

    let invalid: [u8; 9] = [1, 1, 1, 1, 1, 1, 1, 1, 1];
    let invalid = Token::try_consume_bytes(invalid).unwrap_err();
    assert_eq!(invalid, MessageParseError::InvalidTokenLength(9))
  }

  #[test]
  fn delta() {
    let del_4bit = [0b00010000u8];
    let del_4bit = opt::Delta::try_consume_bytes(del_4bit).unwrap();
    assert_eq!(del_4bit, opt::Delta(1));

    let del_1byte = [0b11010000u8, 0b00000000];
    let del_1byte = opt::Delta::try_consume_bytes(del_1byte).unwrap();
    assert_eq!(del_1byte, opt::Delta(13));

    let del_2bytes = [[0b11100000u8].as_ref(), u16::to_be_bytes(12076).as_ref()].concat();
    let del_2bytes = opt::Delta::try_consume_bytes(del_2bytes).unwrap();
    assert_eq!(del_2bytes, opt::Delta(12345));

    let errs = [
      [0b11010000u8].as_ref(),           // delta is 13 but no byte following
      [0b11100000, 0b00000001].as_ref(), // delta is 14 but only 1 byte following
      [].as_ref(),
    ];

    errs.into_iter().for_each(|iter| {
      let del = opt::Delta::try_consume_bytes(iter.to_vec());
      assert_eq!(del, Err(MessageParseError::UnexpectedEndOfStream))
    });
  }

  #[test]
  fn value() {
    let val_1byte: [u8; 2] = [0b00000001, 2];
    let val_1byte = opt::Value::try_consume_bytes(val_1byte).unwrap();
    assert_eq!(val_1byte, opt::Value{len: 1, data: vec![2]});

    let data13bytes = core::iter::repeat(1u8).take(13).collect::<Vec<_>>();
    let val_13bytes = [[0b00001101u8, 0b00000000].as_ref(), &data13bytes].concat();
    let val_13bytes = opt::Value::try_consume_bytes(val_13bytes).unwrap();
    assert_eq!(val_13bytes, opt::Value {len: 13, data: data13bytes});

    let data270bytes = core::iter::repeat(1u8).take(270).collect::<Vec<_>>();
    let val_270bytes = [[0b00001110u8, 0b00000000, 0b00000001].as_ref(), &data270bytes].concat();
    let val_270bytes = opt::Value::try_consume_bytes(val_270bytes).unwrap();
    assert_eq!(val_270bytes, opt::Value{len: 270, data: data270bytes});

    let errs = [
      [0b00000001u8].as_ref(),           // len is 1 but no data following
      [0b00001101u8].as_ref(),           // len value is 13, but no data following
      [0b00001110, 0b00000001].as_ref(), // len value is 14 but only 1 byte following
      [].as_ref(),
    ];

    errs.into_iter().for_each(|iter| {
      let del = opt::Value::try_consume_bytes(iter.to_vec());
      assert_eq!(del, Err(MessageParseError::UnexpectedEndOfStream))
    });
  }

  #[test]
  fn option() {
    let opt_bytes: [u8; 2] = [0b00000001, 0b00000001];
    let opt= opt::Opt::try_consume_bytes(opt_bytes).unwrap();
    assert_eq!(opt, opt::Opt{delta: opt::Delta(0), value: opt::Value {len: 1, data: vec![1]}});

    let opt_bytes: [u8; 5] = [0b00000001, 0b00000001, 0b00010001, 0b00000011, 0b11111111];
    let opt= Vec::<opt::Opt>::try_consume_bytes(opt_bytes).unwrap();
    assert_eq!(opt, vec![
      opt::Opt{delta: opt::Delta(0), value: opt::Value {len: 1, data: vec![1]}},
      opt::Opt{delta: opt::Delta(1), value: opt::Value {len: 1, data: vec![3]}},
    ]);
  }

  #[test]
  fn id() {
    let id_bytes = 34u16.to_be_bytes();
    let id = Id::try_consume_bytes(id_bytes).unwrap();
    assert_eq!(id, Id(34));
  }
}
