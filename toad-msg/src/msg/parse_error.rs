/// Errors encounterable while parsing a message from bytes
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub enum MessageParseError {
  /// Reached end of stream before parsing was finished
  UnexpectedEndOfStream,

  /// Token length was > 8
  InvalidTokenLength(u8),

  /// Error parsing option
  OptParseError(super::opt::parse_error::OptParseError),

  /// The rest of the message contained more bytes than there was capacity for
  PayloadTooLong(usize),

  /// The message type is invalid (see [`Type`] for information & valid values)
  InvalidType(u8),
}

impl MessageParseError {
  /// Shorthand for [`MessageParseError::UnexpectedEndOfStream`]
  pub fn eof() -> Self {
    Self::UnexpectedEndOfStream
  }
}
