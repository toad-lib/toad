/// TODO
#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Mode {
  /// TODO
  Insecure,

  /// TODO
  #[cfg(feature = "std")]
  RawPublicKey,
}
