use kwap_common::Array;
use kwap_msg::{EnumerateOptNumbers, Message, Payload, TryIntoBytes, Type};
#[cfg(feature = "alloc")]
use std_alloc::string::{FromUtf8Error, String};

use crate::config::{self, Config};
use crate::req::Req;

/// Response codes
pub mod code;

/// [`Resp`] that uses [`Vec`] as the backing collection type
///
/// ```
/// use kwap::config::Std;
/// use kwap::resp::Resp;
/// # use kwap_msg::*;
/// # main();
///
/// fn main() {
///   start_server(|req| {
///     let mut resp = Resp::<Std>::for_request(req);
///
///     resp.set_code(kwap::resp::code::CONTENT);
///     resp.set_option(12, Some(50)); // Content-Format: application/json
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
/// fn start_server(f: impl FnOnce(kwap::req::Req<Std>) -> kwap::resp::Resp<Std>) {
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
  /// ```
  /// use kwap::config::{Message, Std};
  /// use kwap::req::Req;
  /// use kwap::resp::Resp;
  ///
  /// // pretend this is an incoming request
  /// let req = Req::<Std>::get("1.1.1.1", 5683, "/hello");
  /// let resp = Resp::<Std>::for_request(req.clone());
  ///
  /// let req_msg = Message::<Std>::from(req);
  /// let resp_msg = Message::<Std>::from(resp);
  ///
  /// // note that Req's default type is CON, so the response will be an ACK.
  /// // this means that the token and id of the response will be the same
  /// // as the incoming request.
  /// assert_eq!(resp_msg.ty, kwap_msg::Type::Ack);
  /// assert_eq!(req_msg.id, resp_msg.id);
  /// assert_eq!(req_msg.token, resp_msg.token);
  /// ```
  pub fn for_request(req: Req<Cfg>) -> Self {
    let req = Message::from(req);

    let msg = Message { ty: match req.ty {
                          | Type::Con => Type::Ack,
                          | Type::Non => Type::Con,
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
  ///
  /// ```
  /// use kwap::config::Std;
  /// use kwap::req::Req;
  /// use kwap::resp::Resp;
  ///
  /// let req = Req::<Std>::get("1.1.1.1", 5683, "/hello");
  ///
  /// // pretend this is an incoming response
  /// let resp = Resp::<Std>::for_request(req);
  ///
  /// let data: Vec<u8> = resp.payload().copied().collect();
  /// ```
  pub fn payload(&self) -> impl Iterator<Item = &u8> {
    self.msg.payload.0.iter()
  }

  /// Get the message type
  ///
  /// See [`kwap_msg::Type`] for more info
  pub fn msg_type(&self) -> kwap_msg::Type {
    self.msg.ty
  }

  /// Get the message id
  ///
  /// See [`kwap_msg::Id`] for more info
  pub fn msg_id(&self) -> kwap_msg::Id {
    self.msg.id
  }

  /// Get the message token
  ///
  /// See [`kwap_msg::Token`] for more info
  pub fn token(&self) -> kwap_msg::Token {
    self.msg.token
  }

  /// Get the payload and attempt to interpret it as an ASCII string
  ///
  /// ```
  /// use kwap::config::Std;
  /// use kwap::req::Req;
  /// use kwap::resp::Resp;
  ///
  /// let req = Req::<Std>::get("1.1.1.1", 5683, "/hello");
  ///
  /// // pretend this is an incoming response
  /// let mut resp = Resp::<Std>::for_request(req);
  /// resp.set_payload("hello!".bytes());
  ///
  /// let data: String = resp.payload_string().unwrap();
  /// ```
  #[cfg(feature = "alloc")]
  pub fn payload_string(&self) -> Result<String, FromUtf8Error> {
    String::from_utf8(self.payload().copied().collect())
  }

  /// Get the response code
  ///
  /// ```
  /// use kwap::config::Std;
  /// use kwap::req::Req;
  /// use kwap::resp::{code, Resp};
  ///
  /// // pretend this is an incoming request
  /// let req = Req::<Std>::get("1.1.1.1", 5683, "/hello");
  /// let resp = Resp::<Std>::for_request(req);
  ///
  /// assert_eq!(resp.code(), code::CONTENT);
  /// ```
  pub fn code(&self) -> kwap_msg::Code {
    self.msg.code
  }

  /// Change the response code
  ///
  /// ```
  /// use kwap::config::Std;
  /// use kwap::req::Req;
  /// use kwap::resp::{code, Resp};
  ///
  /// // pretend this is an incoming request
  /// let req = Req::<Std>::get("1.1.1.1", 5683, "/hello");
  /// let mut resp = Resp::<Std>::for_request(req);
  ///
  /// resp.set_code(code::INTERNAL_SERVER_ERROR);
  /// ```
  pub fn set_code(&mut self, code: kwap_msg::Code) {
    self.msg.code = code;
  }

  /// Add a custom option to the response
  ///
  /// If there was no room in the collection, returns the arguments back as `Some(number, value)`.
  /// Otherwise, returns `None`.
  ///
  /// ```
  /// use kwap::config::Std;
  /// use kwap::req::Req;
  /// use kwap::resp::Resp;
  ///
  /// // pretend this is an incoming request
  /// let req = Req::<Std>::get("1.1.1.1", 5683, "/hello");
  /// let mut resp = Resp::<Std>::for_request(req);
  ///
  /// resp.set_option(17, Some(50)); // Accept: application/json
  /// ```
  pub fn set_option<V: IntoIterator<Item = u8>>(&mut self, number: u32, value: V) -> Option<(u32, V)> {
    if self.opts.is_none() {
      self.opts = Some(Default::default());
    }
    crate::add_option(self.opts.as_mut().unwrap(), number, value)
  }

  /// Add a payload to this response
  ///
  /// ```
  /// use kwap::config::Std;
  /// use kwap::req::Req;
  /// use kwap::resp::Resp;
  ///
  /// // pretend this is an incoming request
  /// let req = Req::<Std>::get("1.1.1.1", 5683, "/hello");
  /// let mut resp = Resp::<Std>::for_request(req);
  ///
  /// // Maybe you have some bytes:
  /// resp.set_payload(vec![1, 2, 3]);
  ///
  /// // Or a string:
  /// resp.set_payload("hello!".bytes());
  /// ```
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

impl<Cfg: Config> TryIntoBytes for Resp<Cfg> {
  type Error = <config::Message<Cfg> as TryIntoBytes>::Error;

  fn try_into_bytes<C: Array<Item = u8>>(self) -> Result<C, Self::Error> {
    config::Message::<Cfg>::from(self).try_into_bytes()
  }
}
