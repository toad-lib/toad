use super::*;

/// Trait for converting a sequence of bytes into some data structure
pub trait TryFromBytes: Sized {
  /// Error type yielded if conversion fails
  type Error;

  /// Try to convert from some sequence of bytes `T`
  /// into `Self`
  fn try_from_bytes<T: IntoIterator<Item = u8>>(bytes: T) -> Result<Self, Self::Error>;
}

/// Trait adding the ability for a _piece_ of a data structure to parse itself by mutating an iterator over bytes.
pub(crate) trait TryConsumeBytes<T>: Sized {
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

  /// Error parsing option
  OptParseError(opt::OptParseError),
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
    let del_4bit = opt::OptDelta::try_consume_bytes(del_4bit).unwrap();
    assert_eq!(del_4bit, opt::OptDelta(1));

    let del_1byte = [0b11010000u8, 0b00000000];
    let del_1byte = opt::OptDelta::try_consume_bytes(del_1byte).unwrap();
    assert_eq!(del_1byte, opt::OptDelta(13));

    let del_2bytes = [[0b11100000u8].as_ref(), u16::to_be_bytes(12076).as_ref()].concat();
    let del_2bytes = opt::OptDelta::try_consume_bytes(del_2bytes).unwrap();
    assert_eq!(del_2bytes, opt::OptDelta(12345));

    let errs = [
      [0b11010000u8].as_ref(),           // delta is 13 but no byte following
      [0b11100000, 0b00000001].as_ref(), // delta is 14 but only 1 byte following
      [].as_ref(),
    ];

    errs.into_iter().for_each(|iter| {
      let del = opt::OptDelta::try_consume_bytes(iter.to_vec());
      assert_eq!(del, Err(opt::OptParseError::UnexpectedEndOfStream))
    });
  }

  #[test]
  fn alloc_opt_value() {
    let val_1byte: [u8; 2] = [0b00000001, 2];
    let val_1byte = opt_alloc::OptValue::try_consume_bytes(val_1byte).unwrap();
    assert_eq!(val_1byte, opt_alloc::OptValue(vec![2]));

    let data13bytes = core::iter::repeat(1u8).take(13).collect::<Vec<_>>();
    let val_13bytes = [[0b00001101u8, 0b00000000].as_ref(), &data13bytes].concat();
    let val_13bytes = opt_alloc::OptValue::try_consume_bytes(val_13bytes).unwrap();
    assert_eq!(val_13bytes, opt_alloc::OptValue(data13bytes));

    let data270bytes = core::iter::repeat(1u8).take(270).collect::<Vec<_>>();
    let val_270bytes = [[0b00001110u8, 0b00000000, 0b00000001].as_ref(), &data270bytes].concat();
    let val_270bytes = opt_alloc::OptValue::try_consume_bytes(val_270bytes).unwrap();
    assert_eq!(val_270bytes, opt_alloc::OptValue(data270bytes));

    let errs = [
      [0b00000001u8].as_ref(),           // len is 1 but no data following
      [0b00001101u8].as_ref(),           // len value is 13, but no data following
      [0b00001110, 0b00000001].as_ref(), // len value is 14 but only 1 byte following
      [].as_ref(),
    ];

    errs.into_iter().for_each(|iter| {
      let del = opt_alloc::OptValue::try_consume_bytes(iter.to_vec());
      assert_eq!(del, Err(opt::OptParseError::UnexpectedEndOfStream))
    });
  }

  #[test]
  fn alloc_opt() {
    let opt_bytes: [u8; 2] = [0b00000001, 0b00000001];
    let opt= opt_alloc::Opt::try_consume_bytes(opt_bytes).unwrap();
    assert_eq!(opt, opt_alloc::Opt{delta: opt::OptDelta(0), value: opt_alloc::OptValue(vec![1])});

    let opt_bytes: [u8; 5] = [0b00000001, 0b00000001, 0b00010001, 0b00000011, 0b11111111];
    let opt= Vec::<opt_alloc::Opt>::try_consume_bytes(opt_bytes).unwrap();
    assert_eq!(opt, vec![
      opt_alloc::Opt{delta: opt::OptDelta(0), value: opt_alloc::OptValue(vec![1])},
      opt_alloc::Opt{delta: opt::OptDelta(1), value: opt_alloc::OptValue(vec![3])},
    ]);
  }

  #[test]
  fn id() {
    let id_bytes = 34u16.to_be_bytes();
    let id = Id::try_consume_bytes(id_bytes).unwrap();
    assert_eq!(id, Id(34));
  }
}
