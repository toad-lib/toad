use toad_common::Cursor;

/// Trait for converting a sequence of bytes into some data structure
pub trait TryFromBytes<A: AsRef<[u8]>>: Sized {
  /// Error type yielded if conversion fails
  type Error;

  /// Try to convert from some sequence of bytes `T`
  /// into `Self`
  fn try_from_bytes(bytes: A) -> Result<Self, Self::Error>;
}

/// Trait adding the ability for a _piece_ of a data structure to parse itself by mutating a cursor over a byte buffer.
pub(crate) trait TryConsumeBytes<A: AsRef<[u8]>>: Sized {
  /// Error type yielded if conversion fails
  type Error;

  /// Try to convert from some sequence of bytes `T`
  /// into `Self`
  fn try_consume_bytes(bytes: &mut Cursor<A>) -> Result<Self, Self::Error>;
}
