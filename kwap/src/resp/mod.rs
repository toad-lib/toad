use kwap_common::Array;
use kwap_msg::{Message, OptNumber, OptValue, Payload, Type};
#[cfg(feature = "alloc")]
use std_alloc::vec::Vec;

use crate::Opt;

/// Response codes
pub mod code;

/// [`Resp`] that uses [`Vec`] as the backing collection type
#[cfg(feature = "alloc")]
pub type VecResp = Resp<Vec<u8>, Vec<u8>, Vec<kwap_msg::Opt<Vec<u8>>>, Vec<Opt<Vec<u8>>>>;

/// TODO: ser/de support
///
/// ```
/// use kwap::resp::Resp;
/// # use kwap_msg::*;
/// # main();
///
/// fn main() {
///   start_server(|req| {
///     let mut resp = Resp::for_request(&req);
///
///     resp.set_code(kwap::resp::code::CONTENT);
///     resp.set_option(12, [50]);
///
///     let payload = r#"""{
///       "foo": "bar",
///       "baz": "quux"
///     }"""#;
///     resp.set_payload(payload.bytes());
///
///     resp
///   });
/// }
///
/// fn start_server(f: impl FnOnce(VecMessage) -> kwap::resp::VecResp) {
///   // servery things
/// # f(VecMessage {
/// #   id: Id(0),
/// #   code: Code::new(0, 1),
/// #   token: Token(Default::default()),
/// #   ty: Type::Con,
/// #   ver: Default::default(),
/// #   opts: vec![],
/// #   payload: Payload(vec![]),
/// #   __optc: Default::default(),
/// # });
/// }
/// ```
#[derive(Clone, Debug)]
pub struct Resp<Bytes: Array<u8>,
 OptBytes: Array<u8> + 'static,
 LowLevelOpts: Array<kwap_msg::Opt<OptBytes>>,
 Opts: Array<Opt<OptBytes>>>
  where for<'a> &'a OptBytes: IntoIterator<Item = &'a u8>,
        for<'a> &'a Bytes: IntoIterator<Item = &'a u8>,
        for<'a> &'a LowLevelOpts: IntoIterator<Item = &'a kwap_msg::Opt<OptBytes>>,
        for<'a> &'a Opts: IntoIterator<Item = &'a Opt<OptBytes>>
{
  msg: Message<Bytes, OptBytes, LowLevelOpts>,
  // TODO: replace with associated list (OptNumber, Opt)
  opts: Opts,
}

impl<Bytes: Array<u8>,
      OptBytes: Array<u8> + 'static,
      LLOpts: Array<kwap_msg::Opt<OptBytes>>,
      Opts: Array<Opt<OptBytes>>> Resp<Bytes, OptBytes, LLOpts, Opts>
  where for<'a> &'a OptBytes: IntoIterator<Item = &'a u8>,
        for<'a> &'a Bytes: IntoIterator<Item = &'a u8>,
        for<'a> &'a LLOpts: IntoIterator<Item = &'a kwap_msg::Opt<OptBytes>>,
        for<'a> &'a Opts: IntoIterator<Item = &'a Opt<OptBytes>>
{
  /// Create a new response for a given request
  ///
  /// TODO: replace msg with Request type
  pub fn for_request(req: &kwap_msg::Message<Bytes, OptBytes, LLOpts>) -> Self {
    let n_opts = req.opts.get_size();

    let msg = Message { ty: match req.ty {
                          | Type::Con => Type::Ack,
                          | _ => req.ty,
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
                        token: req.token,
                        __optc: Default::default() };

    Self { msg,
           opts: Opts::reserve(n_opts) }
  }

  /// Change the response code
  pub fn set_code(&mut self, code: kwap_msg::Code) {
    self.msg.code = code;
  }

  /// Add a custom option to the response
  ///
  /// If there was no room in the collection, returns the arguments back as `Some(number, value)`.
  /// Otherwise, returns `None`.
  pub fn set_option<V: IntoIterator<Item = u8>>(&mut self, number: u32, value: V) -> Option<(u32, V)> {
    let exist = (&self.opts).into_iter().enumerate().find(|(_, o)| o.number.0 == number);

    if let Some((exist_ix, _)) = exist {
      let mut exist = &mut self.opts[exist_ix];
      exist.value = OptValue(value.into_iter().collect());
      return None;
    }

    let n_opts = self.opts.get_size() + 1;
    let no_room = self.opts.max_size().map(|max| max < n_opts).unwrap_or(false);

    if no_room {
      return Some((number, value));
    }

    let opt = Opt::<_> { number: OptNumber(number),
                         value: OptValue(value.into_iter().collect()) };

    self.opts.extend(Some(opt));

    None
  }

  /// Add a payload to this response
  pub fn set_payload<P: IntoIterator<Item = u8>>(&mut self, payload: P) {
    self.msg.payload = Payload(payload.into_iter().collect());
  }
}
