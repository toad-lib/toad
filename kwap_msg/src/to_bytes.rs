use tinyvec::ArrayVec;

pub trait TryIntoBytes {
  type Error;
  fn try_into_bytes<const CAP: usize>(self) -> Result<ArrayVec<[u8; CAP]>, Self::Error>;
}
