use kwap_msg::{EnumerateOptNumbers, Message, Payload, Type};
#[cfg(feature = "alloc")]
use std_alloc::string::{FromUtf8Error, String};

use crate::config::{self, Config};

/// Response codes
pub mod code;

/// [`Resp`] that uses [`Vec`] as the backing collection type
///
/// ```
/// use kwap::{config::Alloc, resp::Resp};
/// # use kwap_msg::*;
/// # main();
///
/// fn main() {
///   start_server(|req| {
///     let mut resp = Resp::<Alloc>::for_request(req);
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
/// fn start_server(f: impl FnOnce(kwap::req::Req<Alloc>) -> kwap::resp::Resp<Alloc>) {
///   // servery things
/// # f(kwap::req::Req::get("foo", 0, ""));
/// }
/// ```
#[derive(Clone, Debug)]
pub struct Resp<Cfg: Config> {
  pub(crate) msg: config::Message<Cfg>,
  opts: Option<Cfg::OptNumbers>,
}

impl<Cfg: Config> Resp<Cfg> {
  /// Create a new response for a given request
  ///
  /// TODO: replace msg with Request type
  pub fn for_request(req: crate::req::Req<Cfg>) -> Self {
    let req = Message::from(req);

    let msg = Message { ty: match req.ty {
                          | Type::Con => Type::Ack,
                          | _ => req.ty,
                        },
                        id: if req.ty == Type::Con {
                          req.id
                        } else {
                          crate::generate_id()
                        },
                        opts: Cfg::Opts::default(),
                        code: code::CONTENT,
                        ver: Default::default(),
                        payload: Payload(Default::default()),
                        token: req.token };

    Self { msg, opts: None }
  }

  /// Get the payload's raw bytes
  pub fn payload(&self) -> impl Iterator<Item = &u8> {
    self.msg.payload.0.iter()
  }

  /// Get the payload and attempt to interpret it as an ASCII string
  #[cfg(feature = "alloc")]
  pub fn payload_string(&self) -> Result<String, FromUtf8Error> {
    String::from_utf8(self.payload().copied().collect())
  }

  /// Get the response code
  pub fn code(&self) -> kwap_msg::Code {
    self.msg.code
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
    if self.opts.is_none() {
      self.opts = Some(Default::default());
    }
    crate::add_option(self.opts.as_mut().unwrap(), number, value)
  }

  /// Add a payload to this response
  pub fn set_payload<P: IntoIterator<Item = u8>>(&mut self, payload: P) {
    self.msg.payload = Payload(payload.into_iter().collect());
  }

  /// Drains the internal associated list of opt number <> opt and converts the numbers into deltas to prepare for message transmission
  fn normalize_opts(&mut self) {
    if let Some(opts) = Option::take(&mut self.opts) {
      self.msg.opts = crate::normalize_opts(opts);
    }
  }
}

impl<Cfg: Config> From<Resp<Cfg>> for config::Message<Cfg> {
  fn from(mut rep: Resp<Cfg>) -> Self {
    rep.normalize_opts();
    rep.msg
  }
}

impl<Cfg: Config> From<config::Message<Cfg>> for Resp<Cfg> {
  fn from(mut msg: config::Message<Cfg>) -> Self {
    let opts = msg.opts.into_iter().enumerate_option_numbers().collect();
    msg.opts = Default::default();

    Self { msg, opts: Some(opts) }
  }
}
