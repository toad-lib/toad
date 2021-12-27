use tinyvec::ArrayVec;

/// Trait allowing fallible conversion into bytes
pub trait TryIntoBytes {
  type Error;

  /// Try to convert into a fixed-capacity collection on the stack
  ///
  /// ```
  /// use kwap_msg::no_alloc as msg;
  /// use msg::TryIntoBytes;
  ///
  /// let message = msg::Message::<0, 0, 0> {
  ///   // ...
  /// # id: msg::Id(0),
  /// # ty: msg::Type(0),
  /// # ver: Default::default(),
  /// # opts: Default::default(),
  /// # payload: msg::Payload(Default::default()),
  /// # token: msg::Token(Default::default()),
  /// # code: msg::Code {class: 0, detail: 1}
  /// };
  ///
  /// let bytes: tinyvec::ArrayVec<[u8; 1024]> = message.try_into_bytes().unwrap();
  /// ```
  fn try_into_bytes<const CAP: usize>(self) -> Result<ArrayVec<[u8; CAP]>, Self::Error>;
}
