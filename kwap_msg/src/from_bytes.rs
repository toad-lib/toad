use tinyvec::ArrayVec;

use crate::*;

/// Trait for converting a sequence of bytes into some data structure
pub trait TryFromBytes<T>: Sized {
  /// Error type yielded if conversion fails
  type Error;

  /// Try to convert from some sequence of bytes `T`
  /// into `Self`
  fn try_from_bytes<I: IntoIterator<Item = T>>(bytes: I) -> Result<Self, Self::Error>;
}

/// Trait adding the ability for a _piece_ of a data structure to parse itself by mutating an iterator over bytes.
pub(crate) trait TryConsumeBytes<I: Iterator<Item = u8>>: Sized
  {
  /// Error type yielded if conversion fails
  type Error;

  /// Try to convert from some sequence of bytes `T`
  /// into `Self`
  fn try_consume_bytes(bytes: &mut I) -> Result<Self, Self::Error>;
}

/// Similar to `TryConsumeBytes` except that the number of bytes to consume is determined by the caller.
pub(crate) trait TryConsumeNBytes<I: Iterator<Item = u8>>: Sized {
  /// Error type yielded if conversion fails
  type Error;

  /// Try to convert from some sequence of bytes `T`
  /// into `Self`
  fn try_consume_n_bytes(n: usize, bytes: &mut I) -> Result<Self, Self::Error>;
}

/// Errors encounterable while parsing an option from bytes
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub enum OptParseError {
  /// Reached end of stream before parsing was finished
  UnexpectedEndOfStream,

  /// Option value was longer than the fixed capacity
  OptionValueTooLong { capacity: usize, actual: usize },

  /// Parsed more options than reserved capacity
  TooManyOptions(usize),

  /// Option Delta was set to 15, which is invalid.
  OptionDeltaReservedValue(u8),

  /// Value Length was set to 15, which is invalid.
  ValueLengthReservedValue(u8),

  /// Not a true failure case; only means we tried to read the payload marker byte (0xFF)
  /// as an option header.
  OptionsExhausted,
}

impl OptParseError {
  pub(super) fn try_next<I>(mut iter: impl Iterator<Item = I>) -> Result<I, Self> {
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

  /// The rest of the message contained more bytes than there was capacity for
  PayloadTooLong(usize),
}

impl MessageParseError {
  pub(super) fn try_next<I>(iter: &mut impl Iterator<Item = I>) -> Result<I, Self> {
    iter.next().ok_or(Self::UnexpectedEndOfStream)
  }
}

impl From<u8> for Byte1 {
  fn from(b: u8) -> Self {
    let ver = b >> 6; // bits 0 & 1
    let ty = b >> 4 & 0b11; // bits 2 & 3
    let tkl = b & 0b1111u8; // last 4 bits

    Byte1 { ver: Version(ver),
            ty: Type(ty),
            tkl }
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
impl From<u8> for Code {
  fn from(b: u8) -> Self {
    let class = b >> 5;
    let detail = b & 0b0011111;

    Code { class, detail }
  }
}
impl<I: Iterator<Item = u8>> TryConsumeBytes<I> for Token {
  type Error = MessageParseError;

  fn try_consume_bytes(bytes: &mut I) -> Result<Self, Self::Error> {
    let token = bytes.into_iter().collect::<ArrayVec<[_; 8]>>();

    Ok(Token(token))
  }
}
impl<I: Iterator<Item = u8>> TryConsumeBytes<I> for OptDelta {
  type Error = OptParseError;

  fn try_consume_bytes(bytes: &mut I) -> Result<Self, Self::Error> {
    let first_byte = Self::Error::try_next(bytes.by_ref())?;
    let delta = first_byte >> 4;
    let delta = opt_len_or_delta(delta, bytes, OptParseError::OptionDeltaReservedValue(15))?;

    Ok(OptDelta(delta))
  }
}

impl<'a, P: Collection<u8>, O: Collection<u8>, Os: Collection<Opt<O>>> TryFromBytes<&'a u8> for Message<P, O, Os>  where
    for<'b> &'b P: IntoIterator<Item = &'b u8>,
    for<'b> &'b O: IntoIterator<Item = &'b u8>,
    for<'b> &'b Os: IntoIterator<Item = &'b Opt<O>>,{
  type Error = MessageParseError;

  fn try_from_bytes<I: IntoIterator<Item = &'a u8>>(bytes: I) -> Result<Self, Self::Error> {
    Self::try_from_bytes(bytes.into_iter().copied())
  }
}

impl<P: Collection<u8>, O: Collection<u8>, Os: Collection<Opt<O>>> TryFromBytes<u8> for Message<P, O, Os>  where
    for<'b> &'b P: IntoIterator<Item = &'b u8>,
    for<'b> &'b O: IntoIterator<Item = &'b u8>,
    for<'b> &'b Os: IntoIterator<Item = &'b Opt<O>>,{
  type Error = MessageParseError;

  fn try_from_bytes<I: IntoIterator<Item = u8>>(bytes: I) -> Result<Self, Self::Error> {
    let mut bytes = bytes.into_iter();

    let Byte1 { tkl, ty, ver } = Self::Error::try_next(&mut bytes)?.into();

    if tkl > 8 {
      return Err(Self::Error::InvalidTokenLength(tkl));
    }

    let code: Code = Self::Error::try_next(&mut bytes)?.into();
    let id: Id = Id::try_consume_bytes(&mut bytes)?;
    let token = Token::try_consume_bytes(&mut bytes.by_ref().take(tkl as usize))?;
    let opts = Os::try_consume_bytes(&mut bytes).map_err(Self::Error::OptParseError)?;
    let payload = Payload(bytes.collect());

    Ok(Message { id,
                 ty,
                 ver,
                 code,
                 token,
                 opts,
                 payload,
                 __optc: Default::default() })
  }
}
