/// Response codes
pub mod code;

/* TODO
use crate::Opt;
use core::mem::MaybeUninit;

use kwap_msg::{Collection as C, Message, EnumerateOptNumbers, OptValue, OptDelta, OptNumber};
use tinyvec::ArrayVec;
#[cfg(feature = "alloc")]
use std_alloc::vec::Vec;

/// [`Resp`] that uses [`Vec`] as the backing collection type
#[cfg(feature = "alloc")]
pub type VecResp = Resp<Vec<u8>, Vec<u8>, Vec<kwap_msg::Opt<Vec<u8>>>, Vec<Opt<Vec<u8>>>>;

/// TODO: ser/de support
#[derive(Clone, Debug)]
pub struct Resp<Bytes: C<u8>, OptBytes: C<u8> + 'static, LowLevelOpts: C<kwap_msg::Opt<OptBytes>>, Opts: C<Opt<OptBytes>>>
  where for<'a> &'a OptBytes: IntoIterator<Item = &'a u8>,
        for<'a> &'a Bytes: IntoIterator<Item = &'a u8>,
        for<'a> &'a LowLevelOpts: IntoIterator<Item = &'a kwap_msg::Opt<OptBytes>>,
        for<'a> &'a Opts: IntoIterator<Item = &'a Opt<OptBytes>>
{
  msg: Message<Bytes, OptBytes, LowLevelOpts>,
  opts: Opts,
}

impl<Bytes: C<u8>, OptBytes: C<u8> + 'static, LLOpts: C<kwap_msg::Opt<OptBytes>>, Opts: C<Opt<OptBytes>>> Resp<Bytes, OptBytes, LLOpts, Opts>
  where for<'a> &'a OptBytes: IntoIterator<Item = &'a u8>,
        for<'a> &'a Bytes: IntoIterator<Item = &'a u8>,
        for<'a> &'a LLOpts: IntoIterator<Item = &'a kwap_msg::Opt<OptBytes>>,
        for<'a> &'a Opts: IntoIterator<Item = &'a Opt<OptBytes>> {

  /// Add a custom option to the response
  pub fn add_option<V: AsRef<[u8]>>(&mut self, number: u32, value: V) -> Option<(u32, V)> {
      let n_opts = self.opts.get_size() + 1;
      let no_room = self.opts.max_size().map(|max| max < n_opts).unwrap_or(false);

      if no_room {
        return Some((number, value));
      }

      let opt = Opt::<_> {
        number: OptNumber(number),
        value: OptValue(value.as_ref().iter().copied().collect()),
      };

      self.opts.extend(Some(opt));

      None
  }
}
*/
