use core::fmt::Write;

use no_std_net::SocketAddr;
use tinyvec::ArrayVec;
use toad_common::*;
use toad_msg::{Id,
               Message,
               Opt,
               OptDelta,
               OptNumber,
               OptValue,
               OptionMap,
               Payload,
               SetOptionError,
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

use crate::platform::{self, PlatformTypes};

/// A CoAP request
///
/// ```
/// use toad::req::Req;
/// use toad::resp::Resp;
/// use toad::std::{dtls, PlatformTypes as Std};
///
/// # main();
/// fn main() {
///   let client = Client::new();
///   let mut req = Req::<Std<dtls::Y>>::post("hello");
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
///   fn send(&self, req: &Req<Std<dtls::Y>>) -> Resp<Std<dtls::Y>> {
///     // send the request
///     # let body = req.payload_str().unwrap().to_string();
///     # let mut resp = Resp::for_request(&req).unwrap();
///     # resp.set_payload(format!("Hello, {}!", body).bytes());
///     # resp
///   }
/// }
/// ```
#[derive(Debug)]
pub struct Req<P: PlatformTypes>(platform::Message<P>);

impl<P: PlatformTypes> PartialEq for Req<P> {
  fn eq(&self, other: &Self) -> bool {
    self.0 == other.0
  }
}

impl<P: PlatformTypes> Clone for Req<P> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<P: PlatformTypes> Req<P> {
  /// Create a request
  pub fn new(method: Method, path: impl AsRef<str>) -> Self {
    let msg = Message { ty: Type::Con,
                        ver: Default::default(),
                        code: method.0,
                        id: Id(Default::default()),
                        opts: Default::default(),
                        payload: Payload(Default::default()),
                        token: Token(Default::default()) };

    let mut self_ = Self(msg);

    self_.set_uri_path(path.as_ref().as_bytes().into_iter().copied())
         .ok();
    self_
  }

  /// TODO
  pub fn set_uri_port(&mut self, port: usize) -> Result<(), platform::toad_msg::opt::SetError<P>> {
    self.set(OptNumber(7), OptValue(port.to_be_bytes()))
  }

  /// TODO
  pub fn set_uri_host<V>(&mut self, v: V) -> Result<(), platform::toad_msg::opt::SetError<P>>
    where V: IntoIterator<Item = u8>
  {
    self.set(OptNumber(3), OptValue(v))
  }

  /// TODO
  pub fn set_uri_path<V>(&mut self, v: V) -> Result<(), platform::toad_msg::opt::SetError<P>>
    where V: IntoIterator<Item = u8>
  {
    self.set(OptNumber(11), OptValue(v))
  }

  /// Updates the Message ID for this request
  ///
  /// ```
  /// use toad::platform;
  /// use toad::req::Req;
  /// use toad::std::{dtls, PlatformTypes as Std};
  /// use toad_msg::{Id, Token};
  ///
  /// let mut req = Req::<Std<dtls::Y>>::get("hello");
  /// req.set_msg_id(Id(0));
  /// req.set_msg_token(Token(Default::default()));
  ///
  /// let msg: platform::Message<Std<dtls::Y>> = req.into();
  /// ```
  pub fn set_msg_id(&mut self, id: Id) {
    self.0.id = id;
  }

  /// Updates the Message Token for this request
  ///
  /// ```
  /// use toad::platform;
  /// use toad::req::Req;
  /// use toad::std::{dtls, PlatformTypes as Std};
  /// use toad_msg::{Id, Token};
  ///
  /// let mut req = Req::<Std<dtls::Y>>::get("hello");
  /// req.set_msg_id(Id(0));
  /// req.set_msg_token(Token(Default::default()));
  ///
  /// let msg: platform::Message<Std<dtls::Y>> = req.into();
  /// ```
  pub fn set_msg_token(&mut self, token: Token) {
    self.0.token = token;
  }

  /// Get the request method
  pub fn method(&self) -> Method {
    Method(self.0.code)
  }

  /// Get the request path (Uri-Path option)
  pub fn path(&self) -> Result<Option<&str>, core::str::Utf8Error> {
    self.get_option(toad_msg::opt::known::no_repeat::PATH)
        .and_then(|o| o.get(0))
        .map(|o| core::str::from_utf8(&o.0).map(Some))
        .unwrap_or(Ok(None))
  }

  /// Get the request type (confirmable, non-confirmable)
  pub fn msg_type(&self) -> toad_msg::Type {
    self.0.ty
  }

  /// Set this request to be non-confirmable
  ///
  /// Some messages do not require an acknowledgement.
  ///
  /// This is particularly true for messages that are repeated regularly for
  /// application requirements, such as repeated readings from a sensor.
  pub fn non(&mut self) -> () {
    self.0.ty = Type::Non;
  }

  /// Get a copy of the message id for this request
  ///
  /// ```
  /// use toad::req::Req;
  /// use toad::std::{dtls, PlatformTypes as Std};
  ///
  /// let req = Req::<Std<dtls::Y>>::get("/hello");
  /// let _msg_id = req.msg_id();
  /// ```
  pub fn msg_id(&self) -> toad_msg::Id {
    self.0.id
  }

  /// Get a copy of the message token for this request
  pub fn msg_token(&self) -> toad_msg::Token {
    self.0.token
  }

  /// Set a custom option for this request not [`known`](toad_msg::opt::known)
  /// to the toad library.
  ///
  /// ```
  /// use toad::req::Req;
  /// use toad::std::{dtls, PlatformTypes as Std};
  /// use toad_msg::{OptNumber, OptValue};
  ///
  /// let mut req = Req::<Std<dtls::Y>>::get("/hello");
  /// req.set(OptNumber(17), OptValue(vec![50])); // Accept: application/json
  /// ```
  pub fn set<V: IntoIterator<Item = u8>>(&mut self,
                                         number: OptNumber,
                                         value: OptValue<V>)
                                         -> Result<(), platform::toad_msg::opt::SetError<P>> {
    self.0.set(number,
               OptValue(value.0
                             .into_iter()
                             .collect::<platform::toad_msg::opt::Bytes<P>>()))
  }

  /// Get the number of values for a given option in this Request
  pub fn count(&mut self, number: OptNumber) -> usize {
    self.as_ref().get(number).map(|v| v.get_size()).unwrap_or(0)
  }

  /// Creates a new GET request
  ///
  /// ```
  /// use toad::req::Req;
  /// use toad::std::{dtls, PlatformTypes as Std};
  ///
  /// let _req = Req::<Std<dtls::Y>>::get("/hello");
  /// ```
  pub fn get(path: impl AsRef<str>) -> Self {
    Self::new(Method::GET, path)
  }

  /// Creates a new POST request
  ///
  /// ```
  /// use toad::req::Req;
  /// use toad::std::{dtls, PlatformTypes as Std};
  ///
  /// let mut req = Req::<Std<dtls::Y>>::post("/hello");
  /// req.set_payload("Hi!".bytes());
  /// ```
  pub fn post(path: impl AsRef<str>) -> Self {
    Self::new(Method::POST, path)
  }

  /// Creates a new PUT request
  ///
  /// ```
  /// use toad::req::Req;
  /// use toad::std::{dtls, PlatformTypes as Std};
  ///
  /// let mut req = Req::<Std<dtls::Y>>::put("/hello");
  /// req.set_payload("Hi!".bytes());
  /// ```
  pub fn put(path: impl AsRef<str>) -> Self {
    Self::new(Method::PUT, path)
  }

  /// Creates a new DELETE request
  ///
  /// ```
  /// use toad::req::Req;
  /// use toad::std::{dtls, PlatformTypes as Std};
  ///
  /// let _req = Req::<Std<dtls::Y>>::delete("/users/john");
  /// ```
  pub fn delete(path: impl AsRef<str>) -> Self {
    Self::new(Method::DELETE, path)
  }

  /// Add a payload to this request
  ///
  /// ```
  /// use toad::req::Req;
  /// use toad::std::{dtls, PlatformTypes as Std};
  ///
  /// let mut req = Req::<Std<dtls::Y>>::put("/hello");
  /// req.set_payload("Hi!".bytes());
  /// ```
  pub fn set_payload<Bytes: ToCoapValue>(&mut self, payload: Bytes) {
    self.0.payload = Payload(payload.to_coap_value::<P::MessagePayload>());
  }

  /// Get the payload's raw bytes
  ///
  /// ```
  /// use toad::req::Req;
  /// use toad::std::{dtls, PlatformTypes as Std};
  ///
  /// let mut req = Req::<Std<dtls::Y>>::post("/hello");
  /// req.set_payload("Hi!".bytes());
  ///
  /// assert!(req.payload().iter().copied().eq("Hi!".bytes()))
  /// ```
  pub fn payload(&self) -> &[u8] {
    &self.0.payload.0
  }

  /// Read an option by its number from the request
  ///
  /// ```
  /// use toad::req::Req;
  /// use toad::std::{dtls, PlatformTypes as Std};
  /// use toad_msg::{OptNumber, OptValue};
  ///
  /// let req = Req::<Std<dtls::Y>>::post("hello");
  /// let path = req.get_option(OptNumber(11)).unwrap();
  /// assert_eq!(path.get(0).unwrap(), &OptValue("hello".as_bytes().to_vec()));
  /// ```
  pub fn get_option(&self, n: OptNumber) -> Option<&<P::MessageOptions as OptionMap>::OptValues> {
    self.0
        .opts
        .iter()
        .find(|(num, _)| **num == n)
        .map(|(_, v)| v)
  }

  /// Get the payload and attempt to interpret it as an ASCII string
  ///
  /// ```
  /// use toad::req::Req;
  /// use toad::std::{dtls, PlatformTypes as Std};
  ///
  /// let mut req = Req::<Std<dtls::Y>>::post("/hello");
  /// req.set_payload("Hi!".bytes());
  ///
  /// assert_eq!(req.payload_str().unwrap(), "Hi!")
  /// ```
  pub fn payload_str(&self) -> Result<&str, core::str::Utf8Error> {
    core::str::from_utf8(self.payload())
  }

  /// Iterate over the options attached to this request
  pub fn opts(
    &self)
    -> impl Iterator<Item = (&OptNumber, &<P::MessageOptions as OptionMap>::OptValues)> {
    self.0.opts.iter()
  }
}

impl<P> AsRef<platform::Message<P>> for Req<P> where P: platform::PlatformTypes
{
  fn as_ref(&self) -> &platform::Message<P> {
    &self.0
  }
}

impl<P> AsMut<platform::Message<P>> for Req<P> where P: platform::PlatformTypes
{
  fn as_mut(&mut self) -> &mut platform::Message<P> {
    &mut self.0
  }
}

impl<P: PlatformTypes> From<Req<P>> for platform::Message<P> {
  fn from(req: Req<P>) -> Self {
    req.0
  }
}

impl<P: PlatformTypes> TryIntoBytes for Req<P> {
  type Error = <platform::Message<P> as TryIntoBytes>::Error;

  fn try_into_bytes<C: Array<Item = u8>>(self) -> Result<C, Self::Error> {
    platform::Message::<P>::from(self).try_into_bytes()
  }
}

impl<P: PlatformTypes> From<platform::Message<P>> for Req<P> {
  fn from(msg: platform::Message<P>) -> Self {
    Self(msg)
  }
}
