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

  /// The rest of the message contained more bytes than there was capacity for
  PayloadTooLong(usize),
}

impl MessageParseError {
  pub(super) fn try_next<I>(iter: &mut impl Iterator<Item = I>) -> Result<I, Self> {
    iter.next().ok_or(Self::UnexpectedEndOfStream)
  }
}
