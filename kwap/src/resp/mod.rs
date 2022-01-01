use crate::Opt;

use kwap_common::Array;
use kwap_msg::{Message, OptValue, OptDelta, OptNumber, Payload, Id};
#[cfg(feature = "alloc")]
use std_alloc::vec::Vec;

/// Response codes
pub mod code;

/// [`Resp`] that uses [`Vec`] as the backing collection type
#[cfg(feature = "alloc")]
pub type VecResp = Resp<Vec<u8>, Vec<u8>, Vec<kwap_msg::Opt<Vec<u8>>>, Vec<Opt<Vec<u8>>>>;

/// TODO: ser/de support
#[derive(Clone, Debug)]
pub struct Resp<Bytes: Array<u8>, OptBytes: Array<u8> + 'static, LowLevelOpts: Array<kwap_msg::Opt<OptBytes>>, Opts: Array<Opt<OptBytes>>>
  where for<'a> &'a OptBytes: IntoIterator<Item = &'a u8>,
        for<'a> &'a Bytes: IntoIterator<Item = &'a u8>,
        for<'a> &'a LowLevelOpts: IntoIterator<Item = &'a kwap_msg::Opt<OptBytes>>,
        for<'a> &'a Opts: IntoIterator<Item = &'a Opt<OptBytes>>
{
  msg: Message<Bytes, OptBytes, LowLevelOpts>,
  opts: Opts,
}

impl<Bytes: Array<u8>, OptBytes: Array<u8> + 'static, LLOpts: Array<kwap_msg::Opt<OptBytes>>, Opts: Array<Opt<OptBytes>>> Resp<Bytes, OptBytes, LLOpts, Opts>
  where for<'a> &'a OptBytes: IntoIterator<Item = &'a u8>,
        for<'a> &'a Bytes: IntoIterator<Item = &'a u8>,
        for<'a> &'a LLOpts: IntoIterator<Item = &'a kwap_msg::Opt<OptBytes>>,
        for<'a> &'a Opts: IntoIterator<Item = &'a Opt<OptBytes>> {

  /// Create a new response for a given request
  ///
  /// TODO: replace msg with Request type
  pub fn new_for_request(req: &kwap_msg::Message<Bytes, OptBytes, LLOpts>) -> Self {
    let msg = Message::<Bytes, OptBytes, LLOpts> {
      ty: if req.ty == Type::Con {
            Type::Ack
          } else if req.ty == Type::Non {
            Type::Non
          },
      id: if req.ty == Type::Con {
            req.id
          } else {
            crate::generate_id()
          },
      opts: LLOpts::default(),
      code: code::CONTENT,
      ver: Default::default(),
      payload: Payload(Default::default()),
      token: req.token.clone(),
      __optc: Default::default(),
    };

    Self {
      msg,
      opts: Opts::reserve(msg.opts.get_size()),
    }
  }

  /// Add a custom option to the response
  ///
  /// If there was no room in the collection, returns the arguments back as `Some(number, value)`.
  /// Otherwise, returns `None`.
  pub fn set_option<V: IntoIterator<Item = u8>>(&mut self, number: u32, value: V) -> Option<(u32, V)> {
      let exist_ix = (&self.opts).into_iter().enumerate().find_map(|(ix, o)| if o.number.0 == number {Some(ix)} else {None});

      if let Some(exist_ix) = exist_ix {
        // add indexmut to array
        let mut exist = &mut self.opts[exist_ix];
        exist.value = OptValue(value.into_iter().collect());
        return None
      }

      let n_opts = self.opts.get_size() + 1;
      let no_room = self.opts.max_size().map(|max| max < n_opts).unwrap_or(false);

      if no_room {
        return Some((number, value));
      }

      let opt = Opt::<_> {
        number: OptNumber(number),
        value: OptValue(value.into_iter().collect()),
      };

      self.opts.extend(Some(opt));

      None
  }
}
