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

/// Peek at the first byte of a byte iterable and interpret as an Option header.
///
/// This converts the iterator into a Peekable and looks at bytes0.
/// Checks if byte 0 is a Payload marker, indicating all options have been read.
pub(super) fn opt_header<I: IntoIterator<Item = u8>>(bytes: I)
                                                     -> Result<(u8, impl Iterator<Item = u8>), OptParseError> {
  let mut bytes = bytes.into_iter().peekable();
  let opt_header = bytes.peek().copied().ok_or(OptParseError::UnexpectedEndOfStream)?;

  if let 0b11111111 = opt_header {
    // This isn't an option, it's the payload!
    Err(OptParseError::OptionsExhausted)?
  }

  Ok((opt_header, bytes))
}

#[doc = include_str!("../docs/parsing/opt_len_or_delta.md")]
pub(super) fn opt_len_or_delta(head: u8,
                               bytes: &mut impl Iterator<Item = u8>,
                               reserved_err: OptParseError)
                               -> Result<u16, OptParseError> {
  if head == 15 {
    Err(reserved_err)?
  }

  match head {
    | 13 => {
      let n = OptParseError::try_next(bytes)?;
      Ok((n as u16) + 13)
    },
    | 14 => bytes.take(2)
                 .collect::<arrayvec::ArrayVec<_, 2>>()
                 .into_inner()
                 .map(|array| u16::from_be_bytes(array) + 269)
                 .map_err(|_| OptParseError::UnexpectedEndOfStream),
    | _ => Ok(head as u16),
  }
}

/// Errors encounterable while parsing an option from bytes
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub enum OptParseError {
  /// Reached end of stream before parsing was finished
  UnexpectedEndOfStream,

  /// Option Delta was set to 15, which is invalid.
  OptionDeltaReservedValue(u8),

  /// Value Length was set to 15, which is invalid.
  ValueLengthReservedValue(u8),

  /// Not a true failure case; only means we tried to read the payload marker byte (0xFF)
  /// as an option header.
  OptionsExhausted,
}

impl OptParseError {
  pub(super) fn try_next<I>(iter: &mut impl Iterator<Item = I>) -> Result<I, Self> {
    iter.next().ok_or(Self::UnexpectedEndOfStream)
  }
}

/// Errors encounterable while parsing a message from bytes
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub enum MessageParseError {
  /// Reached end of stream before parsing was finished
  UnexpectedEndOfStream,

  /// Token length was > 8
  InvalidTokenLength(u8),

  /// Error parsing option
  OptParseError(OptParseError),
}

impl MessageParseError {
  pub(super) fn try_next<I>(iter: &mut impl Iterator<Item = I>) -> Result<I, Self> {
    iter.next().ok_or(Self::UnexpectedEndOfStream)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::no_alloc::*;

  #[test]
  fn byte1() {
    let byte = 0b_01_10_0011u8;
    let byte = Byte1::from(byte);
    assert_eq!(byte,
               Byte1 { ver: Version(1),
                       ty: Type(2),
                       tkl: TokenLength(3) })
  }

  #[test]
  fn id() {
    let id_bytes = 34u16.to_be_bytes();
    let id = Id::try_consume_bytes(id_bytes).unwrap();
    assert_eq!(id, Id(34));
  }

  #[test]
  fn code() {
    let byte = 0b_01_000101u8;
    let code = Code::from(byte);
    assert_eq!(code, Code { class: 2, detail: 5 })
  }

  #[test]
  fn token() {
    let valid_a: [u8; 1] = [0b_00000001u8];
    let valid_a = Token::try_consume_bytes(valid_a).unwrap();
    assert_eq!(valid_a, Token(1));

    let valid_b: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 1];
    let valid_b = Token::try_consume_bytes(valid_b).unwrap();
    assert_eq!(valid_a, valid_b);
  }

  #[test]
  fn delta() {
    let del_4bit = [0b00010000u8];
    let del_4bit = OptDelta::try_consume_bytes(del_4bit).unwrap();
    assert_eq!(del_4bit, OptDelta(1));

    let del_1byte = [0b11010000u8, 0b00000000];
    let del_1byte = OptDelta::try_consume_bytes(del_1byte).unwrap();
    assert_eq!(del_1byte, OptDelta(13));

    let del_2bytes = [[0b11100000u8].as_ref(), u16::to_be_bytes(12076).as_ref()].concat();
    let del_2bytes = OptDelta::try_consume_bytes(del_2bytes).unwrap();
    assert_eq!(del_2bytes, OptDelta(12345));

    let errs = [[0b11010000u8].as_ref(),           // delta is 13 but no byte following
                [0b11100000, 0b00000001].as_ref(), // delta is 14 but only 1 byte following
                [].as_ref()];

    errs.into_iter().for_each(|iter| {
                      let del = OptDelta::try_consume_bytes(iter.to_vec());
                      assert_eq!(del, Err(OptParseError::UnexpectedEndOfStream))
                    });
  }

  mod alloc {
    use crate::{alloc::*, parsing::*};

    #[test]
    fn alloc_opt_value() {
      let val_1byte: [u8; 2] = [0b00000001, 2];
      let val_1byte = OptValue::try_consume_bytes(val_1byte).unwrap();
      assert_eq!(val_1byte, OptValue(vec![2]));

      let data13bytes = core::iter::repeat(1u8).take(13).collect::<Vec<_>>();
      let val_13bytes = [[0b00001101u8, 0b00000000].as_ref(), &data13bytes].concat();
      let val_13bytes = OptValue::try_consume_bytes(val_13bytes).unwrap();
      assert_eq!(val_13bytes, OptValue(data13bytes));

      let data270bytes = core::iter::repeat(1u8).take(270).collect::<Vec<_>>();
      let val_270bytes = [[0b00001110u8, 0b00000000, 0b00000001].as_ref(), &data270bytes].concat();
      let val_270bytes = OptValue::try_consume_bytes(val_270bytes).unwrap();
      assert_eq!(val_270bytes, OptValue(data270bytes));

      let errs = [[0b00000001u8].as_ref(),           // len is 1 but no data following
                  [0b00001101u8].as_ref(),           // len value is 13, but no data following
                  [0b00001110, 0b00000001].as_ref(), // len value is 14 but only 1 byte following
                  [].as_ref()];

      errs.into_iter().for_each(|iter| {
                        let del = OptValue::try_consume_bytes(iter.to_vec());
                        assert_eq!(del, Err(OptParseError::UnexpectedEndOfStream))
                      });
    }
    #[test]
    fn alloc_opt() {
      let opt_bytes: [u8; 2] = [0b00000001, 0b00000001];
      let opt = Opt::try_consume_bytes(opt_bytes).unwrap();
      assert_eq!(opt,
                 Opt { delta: OptDelta(0),
                       value: OptValue(vec![1]) });

      let opt_bytes: [u8; 5] = [0b00000001, 0b00000001, 0b00010001, 0b00000011, 0b11111111];
      let opt = Vec::<Opt>::try_consume_bytes(opt_bytes).unwrap();
      assert_eq!(opt,
                 vec![Opt { delta: OptDelta(0),
                            value: OptValue(vec![1]) },
                      Opt { delta: OptDelta(1),
                            value: OptValue(vec![3]) },]);
    }
  }
}
