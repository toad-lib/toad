use kwap_common::Array;
use kwap_msg::{Message, Opt, Payload, Token, TryIntoBytes, Type};
#[cfg(feature = "alloc")]
use std_alloc::string::{FromUtf8Error, String};

#[doc(hidden)]
pub mod method;
#[doc(inline)]
pub use method::Method;

use crate::config::{self, Config};

/// A CoAP request
///
/// ```
/// use kwap::{config::Alloc, req::Req, resp::Resp};
///
/// # main();
/// fn main() {
///   let client = Client::new();
///   let mut req = Req::<Alloc>::post("coap://myfunnyserver.com", 5632, "hello");
///   req.set_payload("john".bytes());
///
///   let resp = client.send(req);
///   let resp_body = resp.payload_string().unwrap();
///   assert_eq!(resp_body, "Hello, john!".to_string())
/// }
///
/// struct Client {
///   // clienty things
///   # __field: (),
/// }
///
/// impl Client {
///   fn new() -> Self {
///     // create a new client
///     # Self {__field: ()}
///   }
///
///   fn send(&self, req: Req<Alloc>) -> Resp<Alloc> {
///     // send the request
///     # let body = req.payload_string().unwrap();
///     # let mut resp = Resp::for_request(req);
///     # resp.set_payload(format!("Hello, {}!", body).bytes());
///     # resp
///   }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Req<Cfg: Config> {
  pub(crate) msg: config::Message<Cfg>,
  opts: Option<Cfg::OptNumbers>,
}

impl<Cfg: Config> Req<Cfg> {
  fn new<P: AsRef<str>>(method: Method, host: P, port: u16, path: P) -> Self {
    let msg = Message { ty: Type::Con,
                        ver: Default::default(),
                        code: method.0,
                        id: crate::generate_id(),
                        opts: Default::default(),
                        payload: Payload(Default::default()),
                        token: Token(Default::default()) };

    let mut me = Self { msg,
                        opts: Default::default() };

    fn strbytes<'a, S: AsRef<str> + 'a>(s: &'a S) -> impl Iterator<Item = u8> + 'a {
      s.as_ref().as_bytes().iter().copied()
    }

    // Uri-Host
    me.set_option(3, strbytes(&host));

    // Uri-Port
    me.set_option(7, port.to_be_bytes());

    // Uri-Path
    me.set_option(11, strbytes(&path));

    me
  }

  /// Get a copy of the message id for this request
  ///
  /// ```
  /// use kwap::{config::Alloc, req::Req};
  ///
  /// let req = Req::<Alloc>::get("1.1.1.1", 5683, "/hello");
  /// let _msg_id = req.msg_id();
  /// ```
  pub fn msg_id(&self) -> kwap_msg::Id {
    self.msg.id
  }

  /// Add a custom option to this request
  ///
  /// If there was no room in the collection, returns the arguments back as `Some(number, value)`.
  /// Otherwise, returns `None`.
  ///
  /// ```
  /// use kwap::{config::Alloc, req::Req};
  ///
  /// let mut req = Req::<Alloc>::get("1.1.1.1", 5683, "/hello");
  /// req.set_option(17, Some(50)); // Accept: application/json
  /// ```
  pub fn set_option<V: IntoIterator<Item = u8>>(&mut self, number: u32, value: V) -> Option<(u32, V)> {
    if self.opts.is_none() {
      self.opts = Some(Default::default());
    }
    crate::add_option(self.opts.as_mut().unwrap(), number, value)
  }

  /// Creates a new GET request
  ///
  /// ```
  /// use kwap::{config::Alloc, req::Req};
  ///
  /// let _req = Req::<Alloc>::get("1.1.1.1", 5683, "/hello");
  /// ```
  pub fn get<P: AsRef<str>>(host: P, port: u16, path: P) -> Self {
    Self::new(Method::GET, host, port, path)
  }

  /// Creates a new POST request
  ///
  /// ```
  /// use kwap::{config::Alloc, req::Req};
  ///
  /// let mut req = Req::<Alloc>::post("1.1.1.1", 5683, "/hello");
  /// req.set_payload("Hi!".bytes());
  /// ```
  pub fn post<P: AsRef<str>>(host: P, port: u16, path: P) -> Self {
    Self::new(Method::POST, host, port, path)
  }

  /// Creates a new PUT request
  ///
  /// ```
  /// use kwap::{config::Alloc, req::Req};
  ///
  /// let mut req = Req::<Alloc>::put("1.1.1.1", 5683, "/hello");
  /// req.set_payload("Hi!".bytes());
  /// ```
  pub fn put<P: AsRef<str>>(host: P, port: u16, path: P) -> Self {
    Self::new(Method::PUT, host, port, path)
  }

  /// Creates a new DELETE request
  ///
  /// ```
  /// use kwap::{config::Alloc, req::Req};
  ///
  /// let _req = Req::<Alloc>::delete("1.1.1.1", 5683, "/users/john");
  /// ```
  pub fn delete<P: AsRef<str>>(host: P, port: u16, path: P) -> Self {
    Self::new(Method::DELETE, host, port, path)
  }

  /// Add a payload to this request
  ///
  /// ```
  /// use kwap::{config::Alloc, req::Req};
  ///
  /// let mut req = Req::<Alloc>::put("1.1.1.1", 5683, "/hello");
  /// req.set_payload("Hi!".bytes());
  /// ```
  pub fn set_payload<P: IntoIterator<Item = u8>>(&mut self, payload: P) {
    self.msg.payload = Payload(payload.into_iter().collect());
  }

  /// Get the payload's raw bytes
  ///
  /// ```
  /// use kwap::{config::Alloc, req::Req};
  ///
  /// let mut req = Req::<Alloc>::post("1.1.1.1", 5683, "/hello");
  /// req.set_payload("Hi!".bytes());
  ///
  /// assert!(req.payload().copied().eq("Hi!".bytes()))
  /// ```
  pub fn payload(&self) -> impl Iterator<Item = &u8> {
    self.msg.payload.0.iter()
  }

  /// Read an option by its number from the request
  ///
  /// ```
  /// use kwap::{config::Alloc, req::Req};
  ///
  /// let req = Req::<Alloc>::post("1.1.1.1", 5683, "/hello");
  /// let uri_host = req.get_option(3).unwrap();
  /// assert_eq!(uri_host.value.0, "1.1.1.1".bytes().collect::<Vec<_>>());
  /// ```
  pub fn get_option(&self, n: u32) -> Option<&Opt<Cfg::OptBytes>> {
    self.opts
        .as_ref()
        .and_then(|opts| opts.iter().find(|(num, _)| num.0 == n).map(|(_, o)| o))
  }

  /// Get the payload and attempt to interpret it as an ASCII string
  ///
  /// ```
  /// use kwap::{config::Alloc, req::Req};
  ///
  /// let mut req = Req::<Alloc>::post("1.1.1.1", 5683, "/hello");
  /// req.set_payload("Hi!".bytes());
  ///
  /// assert_eq!(req.payload_string().unwrap(), "Hi!".to_string())
  /// ```
  #[cfg(feature = "alloc")]
  pub fn payload_string(&self) -> Result<String, FromUtf8Error> {
    String::from_utf8(self.payload().copied().collect())
  }

  /// Drains the internal associated list of opt number <> opt and converts the numbers into deltas to prepare for message transmission
  fn normalize_opts(&mut self) {
    if let Some(opts) = Option::take(&mut self.opts) {
      self.msg.opts = crate::normalize_opts(opts);
    }
  }
}

impl<Cfg: Config> From<Req<Cfg>> for config::Message<Cfg> {
  fn from(mut req: Req<Cfg>) -> Self {
    req.normalize_opts();
    req.msg
  }
}

impl<Cfg: Config> TryIntoBytes for Req<Cfg> {
  type Error = <config::Message<Cfg> as TryIntoBytes>::Error;

  fn try_into_bytes<C: Array<Item = u8>>(self) -> Result<C, Self::Error> {
    config::Message::<Cfg>::from(self).try_into_bytes()
  }
}
