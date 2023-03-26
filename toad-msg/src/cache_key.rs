use core::fmt::Debug;
use core::hash::{Hash, Hasher};

use toad_array::{AppendCopy, Array};
use toad_hash::Blake2Hasher;

use crate::repeat::{PATH, QUERY};
use crate::{Message, MessageOptions, OptionMap};

/// Default hasher used for [`CacheKey`]
///
/// Hashes:
///  - [Message Code](toad_msg::Message.code)
///  - [Uri-Path](toad_msg::opt::known::no_repeat::HOST)
///  - [Uri-Query](toad_msg::opt::known::no_repeat::HOST)
///  - [Accept](toad_msg::opt::known::no_repeat::ACCEPT)
#[derive(Debug, Clone, Default)]
pub struct DefaultCacheKey(Blake2Hasher);

impl DefaultCacheKey {
  /// Create a new `DefaultCacheKey`
  pub fn new() -> Self {
    Self::default()
  }
}

impl CacheKey for DefaultCacheKey {
  type Hasher = Blake2Hasher;

  fn hasher(&mut self) -> &mut Self::Hasher {
    &mut self.0
  }

  fn add_cache_key<P, O>(&mut self, msg: &Message<P, O>)
    where P: Array<Item = u8> + AppendCopy<u8>,
          O: OptionMap
  {
    msg.code.hash(&mut self.0);
    msg.opts.iter().for_each(|(num, vals)| {
                     if num.include_in_cache_key() {
                       vals.iter().for_each(|v| v.hash(&mut self.0))
                     }
                   });
  }
}

/// The cache key can be used to compare messages for representing
/// the same action against the same resource; for example requests
/// with different IDs but the same method and cache-key affecting options
/// (ex. path, query parameters) will yield the same cache-key.
///
/// Extends [`core::hash::Hash`] with the ability to build a cache-key of a message
/// in the hasher's state.
///
/// [`DefaultCacheKey`] Provides a default implementation.
pub trait CacheKey
  where Self: Sized + Debug
{
  /// Type used to generate hashes
  type Hasher: Hasher;

  #[allow(missing_docs)]
  fn hasher(&mut self) -> &mut Self::Hasher;

  /// Add this message's cache key to the hasher's internal state.
  ///
  /// After invoking this, to get the [`u64`] hash use [`Hasher::finish`].
  ///
  /// Alternately, use [`CacheKey::cache_key`] to go directly to the [`u64`] hash.
  fn add_cache_key<P, O>(&mut self, msg: &Message<P, O>)
    where P: Array<Item = u8> + AppendCopy<u8>,
          O: OptionMap;

  /// Add this message's cache key to the hasher's internal state and yield the [`u64`] hash.
  ///
  /// ```
  /// use core::hash::Hasher;
  ///
  /// use toad_msg::alloc::Message;
  /// use toad_msg::Type::Con;
  /// use toad_msg::{CacheKey, Code, DefaultCacheKey, Id, Token};
  ///
  /// let msg_a = Message::new(Con, Code::GET, Id(1), Token(Default::default()));
  /// let mut ha = DefaultCacheKey::new();
  /// ha.cache_key(&msg_a);
  ///
  /// let msg_b = Message::new(Con, Code::GET, Id(2), Token(Default::default()));
  /// let mut hb = DefaultCacheKey::new();
  /// hb.cache_key(&msg_a);
  ///
  /// assert_eq!(ha.hasher().finish(), hb.hasher().finish());
  /// ```
  fn cache_key<P, O>(&mut self, msg: &Message<P, O>) -> u64
    where P: Array<Item = u8> + AppendCopy<u8>,
          O: OptionMap
  {
    self.add_cache_key(msg);
    self.hasher().finish()
  }
}

impl<T> CacheKey for &mut T where T: CacheKey
{
  type Hasher = T::Hasher;

  fn hasher(&mut self) -> &mut Self::Hasher {
    <T as CacheKey>::hasher(self)
  }

  fn add_cache_key<P, O>(&mut self, msg: &Message<P, O>)
    where P: Array<Item = u8> + AppendCopy<u8>,
          O: OptionMap
  {
    <T as CacheKey>::add_cache_key(self, msg)
  }
}
