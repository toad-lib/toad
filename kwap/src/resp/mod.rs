use kwap_common::Array;
use kwap_msg::{EnumerateOptNumbers, Id, Message, Payload, TryIntoBytes, Type};
#[cfg(feature = "alloc")]
use std_alloc::string::{FromUtf8Error, String};

use crate::platform::{self, Platform};
use crate::req::Req;

/// Response codes
pub mod code;

/// [`Resp`] that uses [`Vec`] as the backing collection type
///
/// ```
/// use kwap::platform::Std;
/// use kwap::resp::Resp;
/// # use kwap_msg::*;
/// # main();
///
/// fn main() {
///   start_server(|req| {
///     let mut resp = Resp::<Std>::for_request(&req).unwrap();
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
/// # f(kwap::req::Req::get("0.0.0.0:1234".parse().unwrap(), ""));
/// }
/// ```
#[derive(Clone, Debug)]
pub struct Resp<P: Platform> {
  pub(crate) msg: platform::Message<P>,
  opts: Option<P::NumberedOptions>,
}

impl<P: Platform> PartialEq for Resp<P> {
    fn eq(&self, other: &Self) -> bool {
      self.msg == other.msg && self.opts == other.opts
    }
}

impl<P: Platform> Resp<P> {
  /// Create a new response for a given request.
  ///
  /// If the request is CONfirmable, this will return Some(ACK).
  ///
  /// If the request is NONconfirmable, this will return Some(NON).
  ///
  /// If the request is EMPTY or RESET, this will return None.
  ///
  /// ```
  /// use kwap::platform::{Message, Std};
  /// use kwap::req::Req;
  /// use kwap::resp::Resp;
  ///
  /// // pretend this is an incoming request
  /// let mut req = Req::<Std>::get("1.1.1.1:5683".parse().unwrap(), "/hello");
  /// req.set_msg_id(kwap_msg::Id(0));
  /// req.set_msg_token(kwap_msg::Token(Default::default()));
  ///
  /// let resp = Resp::<Std>::for_request(&req).unwrap();
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
  pub fn for_request(req: &Req<P>) -> Option<Self> {
    match req.msg_type() {
      | Type::Con => Some(Self::ack(req)),
      | Type::Non => Some(Self::non(req)),
      | _ => None,
    }
  }

  /// Create a response ACKnowledging an incoming request.
  ///
  /// An ack response must be used when you receive
  /// a CON request.
  ///
  /// You may choose to include the response payload in an ACK,
  /// but keep in mind that you might receive duplicate
  /// If you do need to ensure they receive your response,
  /// you
  pub fn ack(req: &Req<P>) -> Self {
    let msg = Message { ty: Type::Ack,
                        id: req.msg_id(),
                        opts: P::MessageOptions::default(),
                        code: code::CONTENT,
                        ver: Default::default(),
                        payload: Payload(Default::default()),
                        token: req.msg_token() };

    Self { msg, opts: None }
  }

  /// Create a CONfirmable response for an incoming request.
  ///
  /// A confirmable response should be used when
  /// you receive a NON request and want to ensure
  /// the client receives your response
  ///
  /// Note that it would be odd to respond to a CON request
  /// with an ACK followed by a CON response, because the client
  /// will keep resending the request until they receive the ACK.
  ///
  /// The `kwap` runtime will continually retry sending this until
  /// an ACKnowledgement from the client is received.
  pub fn con(req: &Req<P>) -> Self {
    let msg = Message { ty: Type::Con,
                        id: Id(Default::default()),
                        opts: P::MessageOptions::default(),
                        code: code::CONTENT,
                        ver: Default::default(),
                        payload: Payload(Default::default()),
                        token: req.msg_token() };

    Self { msg, opts: None }
  }

  /// Create a NONconfirmable response for an incoming request.
  ///
  /// A non-confirmable response should be used when:
  /// - you receive a NON request and don't need to ensure the client received the response
  /// - you receive a CON request and don't need to ensure the client received the response (**you _must_ ACK this type of request separately**)
  pub fn non(req: &Req<P>) -> Self {
    let msg = Message { ty: Type::Non,
                        id: Id(Default::default()),
                        opts: P::MessageOptions::default(),
                        code: code::CONTENT,
                        ver: Default::default(),
                        payload: Payload(Default::default()),
                        token: req.msg_token() };

    Self { msg, opts: None }
  }

  /// Get the payload's raw bytes
  ///
  /// ```
  /// use kwap::platform::Std;
  /// use kwap::req::Req;
  /// use kwap::resp::Resp;
  ///
  /// let req = Req::<Std>::get("1.1.1.1:5683".parse().unwrap(), "/hello");
  ///
  /// // pretend this is an incoming response
  /// let resp = Resp::<Std>::for_request(&req).unwrap();
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
  /// use kwap::platform::Std;
  /// use kwap::req::Req;
  /// use kwap::resp::Resp;
  ///
  /// let req = Req::<Std>::get("1.1.1.1:5683".parse().unwrap(), "/hello");
  ///
  /// // pretend this is an incoming response
  /// let mut resp = Resp::<Std>::for_request(&req).unwrap();
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
  /// use kwap::platform::Std;
  /// use kwap::req::Req;
  /// use kwap::resp::{code, Resp};
  ///
  /// // pretend this is an incoming request
  /// let req = Req::<Std>::get("1.1.1.1:5683".parse().unwrap(), "/hello");
  /// let resp = Resp::<Std>::for_request(&req).unwrap();
  ///
  /// assert_eq!(resp.code(), code::CONTENT);
  /// ```
  pub fn code(&self) -> kwap_msg::Code {
    self.msg.code
  }

  /// Change the response code
  ///
  /// ```
  /// use kwap::platform::Std;
  /// use kwap::req::Req;
  /// use kwap::resp::{code, Resp};
  ///
  /// // pretend this is an incoming request
  /// let req = Req::<Std>::get("1.1.1.1:5683".parse().unwrap(), "/hello");
  /// let mut resp = Resp::<Std>::for_request(&req).unwrap();
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
  /// use kwap::platform::Std;
  /// use kwap::req::Req;
  /// use kwap::resp::Resp;
  ///
  /// // pretend this is an incoming request
  /// let req = Req::<Std>::get("1.1.1.1:5683".parse().unwrap(), "/hello");
  /// let mut resp = Resp::<Std>::for_request(&req).unwrap();
  ///
  /// resp.set_option(17, Some(50)); // Accept: application/json
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

  /// Add a payload to this response
  ///
  /// ```
  /// use kwap::platform::Std;
  /// use kwap::req::Req;
  /// use kwap::resp::Resp;
  ///
  /// // pretend this is an incoming request
  /// let req = Req::<Std>::get("1.1.1.1:5683".parse().unwrap(), "/hello");
  /// let mut resp = Resp::<Std>::for_request(&req).unwrap();
  ///
  /// // Maybe you have some bytes:
  /// resp.set_payload(vec![1, 2, 3]);
  ///
  /// // Or a string:
  /// resp.set_payload("hello!".bytes());
  /// ```
  pub fn set_payload<Bytes: IntoIterator<Item = u8>>(&mut self, payload: Bytes) {
    self.msg.payload = Payload(payload.into_iter().collect());
  }

  /// Drains the internal associated list of opt number <> opt and converts the numbers into deltas to prepare for message transmission
  fn normalize_opts(&mut self) {
    if let Some(opts) = Option::take(&mut self.opts) {
      self.msg.opts = crate::option::normalize(opts);
    }
  }
}

impl<P: Platform> From<Resp<P>> for platform::Message<P> {
  fn from(mut rep: Resp<P>) -> Self {
    rep.normalize_opts();
    rep.msg
  }
}

impl<P: Platform> From<platform::Message<P>> for Resp<P> {
  fn from(mut msg: platform::Message<P>) -> Self {
    let opts = msg.opts.into_iter().enumerate_option_numbers().collect();
    msg.opts = Default::default();

    Self { msg,
           opts: Some(opts) }
  }
}

impl<P: Platform> TryIntoBytes for Resp<P> {
  type Error = <platform::Message<P> as TryIntoBytes>::Error;

  fn try_into_bytes<C: Array<Item = u8>>(self) -> Result<C, Self::Error> {
    platform::Message::<P>::from(self).try_into_bytes()
  }
}
