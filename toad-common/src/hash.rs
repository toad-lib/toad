use core::fmt::Debug;
use core::hash::Hasher;

use blake2::digest::typenum::U8;
use blake2::{Blake2b, Digest};

/// Heap-allocless [`Hasher`] implementation that uses
/// the [`blake2`] algo to generate a 64 bit hash.
///
/// ```
/// use core::hash::{Hash, Hasher};
///
/// use toad_common::hash::Blake2Hasher;
///
/// let mut hasher_a = Blake2Hasher::new();
/// let mut hasher_b = Blake2Hasher::new();
///
/// let bytes = core::iter::repeat(0u8..255).take(512)
///                                         .flatten()
///                                         .collect::<Vec<u8>>();
///
/// bytes.hash(&mut hasher_a);
/// bytes.hash(&mut hasher_b);
/// assert_eq!(hasher_a.finish(), hasher_b.finish());
///
/// "hello".hash(&mut hasher_a);
/// "hello".hash(&mut hasher_b);
/// assert_eq!(hasher_a.finish(), hasher_b.finish());
///
/// 123_u16.hash(&mut hasher_a);
/// "not 123!".hash(&mut hasher_b);
/// assert_ne!(hasher_a.finish(), hasher_b.finish());
/// ```
#[derive(Default, Clone)]
pub struct Blake2Hasher(Blake2b<U8>);

impl Blake2Hasher {
  /// Create a new `Blake2Hasher`
  pub fn new() -> Self {
    Self::default()
  }
}

impl Debug for Blake2Hasher {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_tuple("Blake2Hasher")
     .field(&"<Blake2bCore<U8>>")
     .finish()
  }
}

impl Hasher for Blake2Hasher {
  fn finish(&self) -> u64 {
    u64::from_be_bytes(self.0.clone().finalize().into())
  }

  fn write(&mut self, bytes: &[u8]) {
    self.0.update(bytes);
  }
}
