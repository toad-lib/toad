#[cfg(feature = "alloc")]
use std_alloc::string::{FromUtf8Error, String};
use toad_array::Array;
use toad_msg::{Id, Message, Payload, TryIntoBytes, Type};

use crate::platform::{self, PlatformTypes};
use crate::req::Req;

/// Response codes
pub mod code;

/// [`Resp`] that uses [`Vec`] as the backing collection type
///
/// ```
/// use toad::resp::Resp;
/// use toad::std::{dtls, PlatformTypes as Std};
/// # use toad_msg::*;
/// # main();
///
/// fn main() {
///   start_server(|req| {
///     let mut resp = Resp::<Std<dtls::Y>>::for_request(&req).unwrap();
///
///     resp.set_code(toad::resp::code::CONTENT);
///     resp.msg_mut()
///         .set_content_format(toad_msg::ContentFormat::Json); // Content-Format: application/json
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
/// fn start_server(f: impl FnOnce(toad::req::Req<Std<dtls::Y>>) -> toad::resp::Resp<Std<dtls::Y>>) {
///   // servery things
/// # f(toad::req::Req::get( ""));
/// }
/// ```
pub struct Resp<P>(platform::Message<P>) where P: PlatformTypes;

impl<P> AsRef<platform::Message<P>> for Resp<P> where P: PlatformTypes
{
  fn as_ref(&self) -> &platform::Message<P> {
    &self.0
  }
}

impl<P> AsMut<platform::Message<P>> for Resp<P> where P: PlatformTypes
{
  fn as_mut(&mut self) -> &mut platform::Message<P> {
    &mut self.0
  }
}

impl<P> core::fmt::Debug for Resp<P> where P: PlatformTypes
{
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_tuple("Resp").field(&self.0).finish()
  }
}

impl<P> Clone for Resp<P> where P: PlatformTypes
{
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<P> PartialEq for Resp<P> where P: PlatformTypes
{
  fn eq(&self, other: &Self) -> bool {
    self.0 == other.0
  }
}

impl<P: PlatformTypes> Resp<P> {
  /// Obtain a reference to the inner message
  pub fn msg(&self) -> &platform::Message<P> {
    &self.0
  }

  /// Obtain a mutable reference to the inner message
  pub fn msg_mut(&mut self) -> &mut platform::Message<P> {
    &mut self.0
  }

  /// Create a new response for a given request.
  ///
  /// If the request is CONfirmable, this will return Some(ACK).
  ///
  /// If the request is NONconfirmable, this will return Some(NON).
  ///
  /// If the request is EMPTY or RESET, this will return None.
  ///
  /// ```
  /// use toad::platform::Message;
  /// use toad::req::Req;
  /// use toad::resp::Resp;
  /// use toad::std::{dtls, PlatformTypes as Std};
  ///
  /// // pretend this is an incoming request
  /// let mut req = Req::<Std<dtls::Y>>::get("/hello");
  /// req.msg_mut().id = toad_msg::Id(0);
  /// req.msg_mut().token = toad_msg::Token(Default::default());
  ///
  /// let resp = Resp::<Std<dtls::Y>>::for_request(&req).unwrap();
  ///
  /// let req_msg = Message::<Std<dtls::Y>>::from(req);
  /// let resp_msg = Message::<Std<dtls::Y>>::from(resp);
  ///
  /// // note that Req's default type is CON, so the response will be an ACK.
  /// // this means that the token and id of the response will be the same
  /// // as the incoming request.
  /// assert_eq!(resp_msg.ty, toad_msg::Type::Ack);
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
                        id: req.msg().id,
                        opts: P::MessageOptions::default(),
                        code: code::CONTENT,
                        ver: Default::default(),
                        payload: Payload(Default::default()),
                        token: req.msg().token };

    Self(msg)
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
  /// The `toad` runtime will continually retry sending this until
  /// an ACKnowledgement from the client is received.
  pub fn con(req: &Req<P>) -> Self {
    let msg = Message { ty: Type::Con,
                        id: Id(Default::default()),
                        opts: P::MessageOptions::default(),
                        code: code::CONTENT,
                        ver: Default::default(),
                        payload: Payload(Default::default()),
                        token: req.msg().token };

    Self(msg)
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
                        token: req.msg().token };

    Self(msg)
  }

  /// Get the payload's raw bytes
  ///
  /// ```
  /// use toad::req::Req;
  /// use toad::resp::Resp;
  /// use toad::std::{dtls, PlatformTypes as Std};
  ///
  /// let req = Req::<Std<dtls::Y>>::get("/hello");
  ///
  /// // pretend this is an incoming response
  /// let resp = Resp::<Std<dtls::Y>>::for_request(&req).unwrap();
  ///
  /// let data: Vec<u8> = resp.payload().copied().collect();
  /// ```
  pub fn payload(&self) -> impl Iterator<Item = &u8> {
    self.0.payload.0.iter()
  }

  /// Get the message type
  ///
  /// See [`toad_msg::Type`] for more info
  pub fn msg_type(&self) -> toad_msg::Type {
    self.0.ty
  }

  /// Get the message id
  ///
  /// See [`toad_msg::Id`] for more info
  pub fn msg_id(&self) -> toad_msg::Id {
    self.0.id
  }

  /// Get the message token
  ///
  /// See [`toad_msg::Token`] for more info
  pub fn token(&self) -> toad_msg::Token {
    self.0.token
  }

  /// Get the payload and attempt to interpret it as an ASCII string
  ///
  /// ```
  /// use toad::req::Req;
  /// use toad::resp::Resp;
  /// use toad::std::{dtls, PlatformTypes as Std};
  ///
  /// let req = Req::<Std<dtls::Y>>::get("/hello");
  ///
  /// // pretend this is an incoming response
  /// let mut resp = Resp::<Std<dtls::Y>>::for_request(&req).unwrap();
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
  /// use toad::req::Req;
  /// use toad::resp::{code, Resp};
  /// use toad::std::{dtls, PlatformTypes as Std};
  ///
  /// // pretend this is an incoming request
  /// let req = Req::<Std<dtls::Y>>::get("/hello");
  /// let resp = Resp::<Std<dtls::Y>>::for_request(&req).unwrap();
  ///
  /// assert_eq!(resp.code(), code::CONTENT);
  /// ```
  pub fn code(&self) -> toad_msg::Code {
    self.0.code
  }

  /// Change the response code
  ///
  /// ```
  /// use toad::req::Req;
  /// use toad::resp::{code, Resp};
  /// use toad::std::{dtls, PlatformTypes as Std};
  ///
  /// // pretend this is an incoming request
  /// let req = Req::<Std<dtls::Y>>::get("/hello");
  /// let mut resp = Resp::<Std<dtls::Y>>::for_request(&req).unwrap();
  ///
  /// resp.set_code(code::INTERNAL_SERVER_ERROR);
  /// ```
  pub fn set_code(&mut self, code: toad_msg::Code) {
    self.0.code = code;
  }

  /// Add a payload to this response
  ///
  /// ```
  /// use toad::req::Req;
  /// use toad::resp::Resp;
  /// use toad::std::{dtls, PlatformTypes as Std};
  ///
  /// // pretend this is an incoming request
  /// let req = Req::<Std<dtls::Y>>::get("/hello");
  /// let mut resp = Resp::<Std<dtls::Y>>::for_request(&req).unwrap();
  ///
  /// // Maybe you have some bytes:
  /// resp.set_payload(vec![1, 2, 3]);
  ///
  /// // Or a string:
  /// resp.set_payload("hello!".bytes());
  /// ```
  pub fn set_payload<Bytes: IntoIterator<Item = u8>>(&mut self, payload: Bytes) {
    self.0.payload = Payload(payload.into_iter().collect());
  }
}

impl<P: PlatformTypes> From<Resp<P>> for platform::Message<P> {
  fn from(rep: Resp<P>) -> Self {
    rep.0
  }
}

impl<P: PlatformTypes> From<platform::Message<P>> for Resp<P> {
  fn from(msg: platform::Message<P>) -> Self {
    Self(msg)
  }
}

impl<P: PlatformTypes> TryIntoBytes for Resp<P> {
  type Error = <platform::Message<P> as TryIntoBytes>::Error;

  fn try_into_bytes<C: Array<Item = u8>>(self) -> Result<C, Self::Error> {
    platform::Message::<P>::from(self).try_into_bytes()
  }
}
