use kwap_common::Array;
use kwap_msg::{EnumerateOptNumbers,
               Id,
               Message,
               Opt,
               OptNumber,
               Payload,
               Token,
               TryIntoBytes,
               Type};

use crate::ToCoapValue;

/// Request methods
pub mod method;

#[doc(inline)]
pub use method::Method;

/// Request builder
pub mod builder;

#[doc(inline)]
pub use builder::*;

use crate::platform::{self, Platform};

/// A CoAP request
///
/// ```
/// use kwap::platform::Std;
/// use kwap::req::Req;
/// use kwap::resp::Resp;
///
/// # main();
/// fn main() {
///   let client = Client::new();
///   let mut req = Req::<Std>::post("coap://myfunnyserver.com", 5632, "hello");
///   req.set_payload("john".bytes());
///
///   let resp = client.send(&req);
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
///   fn send(&self, req: &Req<Std>) -> Resp<Std> {
///     // send the request
///     # let body = req.payload_str().unwrap().to_string();
///     # let mut resp = Resp::for_request(&req).unwrap();
///     # resp.set_payload(format!("Hello, {}!", body).bytes());
///     # resp
///   }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Req<P: Platform> {
  pub(crate) msg: platform::Message<P>,
  pub(crate) id: Option<Id>,
  pub(crate) token: Option<Token>,
  pub(crate) opts: Option<P::NumberedOptions>,
}

impl<P: Platform> Req<P> {
  /// Create a request
  pub fn new(method: Method, host: impl AsRef<str>, port: u16, path: impl AsRef<str>) -> Self {
    let msg = Message { ty: Type::Con,
                        ver: Default::default(),
                        code: method.0,
                        id: Id(Default::default()),
                        opts: Default::default(),
                        payload: Payload(Default::default()),
                        token: Token(Default::default()) };

    let mut me = Self { msg,
                        opts: Default::default(),
                        id: None,
                        token: None };

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

  /// Updates the Message ID for this request
  ///
  /// NOTE:
  /// attempting to convert a request into a [`kwap_msg::Message`] without
  /// first calling `set_msg_id` and `set_msg_token` will panic.
  ///
  /// These 2 methods will always be invoked for you by the kwap runtime.
  ///
  /// ```should_panic
  /// use kwap::platform;
  /// use kwap::platform::Std;
  /// use kwap::req::Req;
  ///
  /// let req = Req::<Std>::get("127.0.0.1", 5683, "hello");
  /// // Panics!!
  /// let msg: platform::Message<Std> = req.into();
  /// ```
  ///
  /// ```
  /// use kwap::platform;
  /// use kwap::platform::Std;
  /// use kwap::req::Req;
  /// use kwap_msg::{Id, Token};
  ///
  /// let mut req = Req::<Std>::get("127.0.0.1", 5683, "hello");
  /// req.set_msg_id(Id(0));
  /// req.set_msg_token(Token(Default::default()));
  ///
  /// // Works B)
  /// let msg: platform::Message<Std> = req.into();
  /// ```
  pub fn set_msg_id(&mut self, id: Id) {
    self.id = Some(id);
  }

  /// Updates the Message Token for this request
  ///
  /// NOTE:
  /// attempting to convert a request into a [`kwap_msg::Message`] without
  /// first calling `set_msg_id` and `set_msg_token` will panic.
  ///
  /// These 2 methods will always be invoked for you by the kwap runtime.
  ///
  /// ```should_panic
  /// use kwap::platform;
  /// use kwap::platform::Std;
  /// use kwap::req::Req;
  ///
  /// let req = Req::<Std>::get("127.0.0.1", 5683, "hello");
  /// // Panics!!
  /// let msg: platform::Message<Std> = req.into();
  /// ```
  ///
  /// ```
  /// use kwap::platform;
  /// use kwap::platform::Std;
  /// use kwap::req::Req;
  /// use kwap_msg::{Id, Token};
  ///
  /// let mut req = Req::<Std>::get("127.0.0.1", 5683, "hello");
  /// req.set_msg_id(Id(0));
  /// req.set_msg_token(Token(Default::default()));
  ///
  /// // Works B)
  /// let msg: platform::Message<Std> = req.into();
  /// ```
  pub fn set_msg_token(&mut self, token: Token) {
    self.token = Some(token);
  }

  /// Get the request method
  pub fn method(&self) -> Method {
    Method(self.msg.code)
  }

  /// Get the request path (Uri-Path option)
  pub fn path(&self) -> Result<Option<&str>, core::str::Utf8Error> {
    self.get_option(11)
        .map(|o| core::str::from_utf8(&o.value.0).map(Some))
        .unwrap_or(Ok(None))
  }

  /// Get the request type (confirmable, non-confirmable)
  pub fn msg_type(&self) -> kwap_msg::Type {
    self.msg.ty
  }

  /// Set this request to be non-confirmable
  ///
  /// Some messages do not require an acknowledgement.
  ///
  /// This is particularly true for messages that are repeated regularly for
  /// application requirements, such as repeated readings from a sensor.
  pub fn non(&mut self) -> () {
    self.msg.ty = Type::Non;
  }

  /// Get a copy of the message id for this request
  ///
  /// ```
  /// use kwap::platform::Std;
  /// use kwap::req::Req;
  ///
  /// let req = Req::<Std>::get("1.1.1.1", 5683, "/hello");
  /// let _msg_id = req.msg_id();
  /// ```
  pub fn msg_id(&self) -> kwap_msg::Id {
    self.id.unwrap_or(self.msg.id)
  }

  /// Get a copy of the message token for this request
  pub fn msg_token(&self) -> kwap_msg::Token {
    self.token.unwrap_or(self.msg.token)
  }

  /// Add a custom option to this request
  ///
  /// If there was no room in the collection, returns the arguments back as `Some(number, value)`.
  /// Otherwise, returns `None`.
  ///
  /// ```
  /// use kwap::platform::Std;
  /// use kwap::req::Req;
  ///
  /// let mut req = Req::<Std>::get("1.1.1.1", 5683, "/hello");
  /// req.set_option(17, Some(50)); // Accept: application/json
  /// ```
  pub fn set_option<V: IntoIterator<Item = u8>>(&mut self,
                                                number: u32,
                                                value: V)
                                                -> Option<(u32, V)> {
    if self.opts.is_none() {
      self.opts = Some(Default::default());
    }

    crate::option::add(self.opts.as_mut().unwrap(), false, number, value)
  }

  /// Add an instance of a repeatable option to the request.
  pub fn add_option<V: IntoIterator<Item = u8>>(&mut self,
                                                number: u32,
                                                value: V)
                                                -> Option<(u32, V)> {
    if self.opts.is_none() {
      self.opts = Some(Default::default());
    }

    crate::option::add(self.opts.as_mut().unwrap(), true, number, value)
  }

  /// Creates a new GET request
  ///
  /// ```
  /// use kwap::platform::Std;
  /// use kwap::req::Req;
  ///
  /// let _req = Req::<Std>::get("1.1.1.1", 5683, "/hello");
  /// ```
  pub fn get(host: impl AsRef<str>, port: u16, path: impl AsRef<str>) -> Self {
    Self::new(Method::GET, host, port, path)
  }

  /// Creates a new POST request
  ///
  /// ```
  /// use kwap::platform::Std;
  /// use kwap::req::Req;
  ///
  /// let mut req = Req::<Std>::post("1.1.1.1", 5683, "/hello");
  /// req.set_payload("Hi!".bytes());
  /// ```
  pub fn post(host: impl AsRef<str>, port: u16, path: impl AsRef<str>) -> Self {
    Self::new(Method::POST, host, port, path)
  }

  /// Creates a new PUT request
  ///
  /// ```
  /// use kwap::platform::Std;
  /// use kwap::req::Req;
  ///
  /// let mut req = Req::<Std>::put("1.1.1.1", 5683, "/hello");
  /// req.set_payload("Hi!".bytes());
  /// ```
  pub fn put(host: impl AsRef<str>, port: u16, path: impl AsRef<str>) -> Self {
    Self::new(Method::PUT, host, port, path)
  }

  /// Creates a new DELETE request
  ///
  /// ```
  /// use kwap::platform::Std;
  /// use kwap::req::Req;
  ///
  /// let _req = Req::<Std>::delete("1.1.1.1", 5683, "/users/john");
  /// ```
  pub fn delete(host: impl AsRef<str>, port: u16, path: impl AsRef<str>) -> Self {
    Self::new(Method::DELETE, host, port, path)
  }

  /// Add a payload to this request
  ///
  /// ```
  /// use kwap::platform::Std;
  /// use kwap::req::Req;
  ///
  /// let mut req = Req::<Std>::put("1.1.1.1", 5683, "/hello");
  /// req.set_payload("Hi!".bytes());
  /// ```
  pub fn set_payload<Bytes: ToCoapValue>(&mut self, payload: Bytes) {
    self.msg.payload = Payload(payload.to_coap_value::<P::MessagePayload>());
  }

  /// Get the payload's raw bytes
  ///
  /// ```
  /// use kwap::platform::Std;
  /// use kwap::req::Req;
  ///
  /// let mut req = Req::<Std>::post("1.1.1.1", 5683, "/hello");
  /// req.set_payload("Hi!".bytes());
  ///
  /// assert!(req.payload().iter().copied().eq("Hi!".bytes()))
  /// ```
  pub fn payload(&self) -> &[u8] {
    &self.msg.payload.0
  }

  /// Read an option by its number from the request
  ///
  /// ```
  /// use kwap::platform::Std;
  /// use kwap::req::Req;
  ///
  /// let req = Req::<Std>::post("1.1.1.1", 5683, "/hello");
  /// let uri_host = req.get_option(3).unwrap();
  /// assert_eq!(uri_host.value.0, "1.1.1.1".bytes().collect::<Vec<_>>());
  /// ```
  pub fn get_option(&self, n: u32) -> Option<&Opt<P::MessageOptionBytes>> {
    self.opts
        .as_ref()
        .and_then(|opts| opts.iter().find(|(num, _)| num.0 == n).map(|(_, o)| o))
  }

  /// Get the payload and attempt to interpret it as an ASCII string
  ///
  /// ```
  /// use kwap::platform::Std;
  /// use kwap::req::Req;
  ///
  /// let mut req = Req::<Std>::post("1.1.1.1", 5683, "/hello");
  /// req.set_payload("Hi!".bytes());
  ///
  /// assert_eq!(req.payload_str().unwrap(), "Hi!")
  /// ```
  pub fn payload_str(&self) -> Result<&str, core::str::Utf8Error> {
    core::str::from_utf8(self.payload())
  }

  /// Drains the internal associated list of opt number <> opt and converts the numbers into deltas to prepare for message transmission
  fn normalize_opts(&mut self) {
    if let Some(opts) = Option::take(&mut self.opts) {
      self.msg.opts = crate::option::normalize(opts);
    }
  }

  /// Iterate over the options attached to this request
  pub fn opts(&self) -> impl Iterator<Item = &(OptNumber, Opt<P::MessageOptionBytes>)> {
    self.opts.iter().flat_map(|opts| opts.iter())
  }
}

impl<P: Platform> From<Req<P>> for platform::Message<P> {
  fn from(mut req: Req<P>) -> Self {
    req.normalize_opts();
    req.msg.id = req.id.expect("Request ID was None");
    req.msg.token = req.token.expect("Request Token was None");
    req.msg
  }
}

impl<P: Platform> TryIntoBytes for Req<P> {
  type Error = <platform::Message<P> as TryIntoBytes>::Error;

  fn try_into_bytes<C: Array<Item = u8>>(self) -> Result<C, Self::Error> {
    platform::Message::<P>::from(self).try_into_bytes()
  }
}

impl<P: Platform> From<platform::Message<P>> for Req<P> {
  fn from(mut msg: platform::Message<P>) -> Self {
    let opts = msg.opts.into_iter().enumerate_option_numbers().collect();
    msg.opts = Default::default();
    let (id, token) = (msg.id, msg.token);

    Self { msg,
           opts: Some(opts),
           id: Some(id),
           token: Some(token) }
  }
}
