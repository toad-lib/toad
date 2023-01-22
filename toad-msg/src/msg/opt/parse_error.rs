/// Errors encounterable while parsing an option from bytes
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub enum OptParseError {
  /// Reached end of stream before parsing was finished
  UnexpectedEndOfStream,

  /// Option value was longer than the fixed capacity
  #[allow(missing_docs)]
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
  /// Shorthand for [`OptParseError::UnexpectedEndOfStream`]
  pub fn eof() -> Self {
    Self::UnexpectedEndOfStream
  }
}
