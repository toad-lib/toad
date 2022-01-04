use core::ops::{Deref, DerefMut};

use kwap_common::Array;
use kwap_msg::{Message, Opt, OptNumber, Payload, Token, Type};
#[cfg(feature = "alloc")]
use std_alloc::{string::{FromUtf8Error, String},
                vec::Vec};

#[doc(hidden)]
pub mod method;
#[doc(inline)]
pub use method::Method;

/// A request that uses `Vec`
///
/// ```
/// use kwap::{req::Req, resp::Resp};
///
/// # main();
/// fn main() {
///   let client = Client::new();
///   let mut req = Req::post("coap://myfunnyserver.com/hello");
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
///   fn send(&self, req: Req) -> Resp {
///     // send the request
///     # let body = req.payload_string().unwrap();
///     # let mut resp = Resp::for_request(req);
///     # resp.set_payload(format!("Hello, {}!", body).bytes());
///     # resp
///   }
/// }
/// ```
#[cfg(feature = "alloc")]
#[cfg_attr(any(docsrs, feature = "docs"), doc(cfg(feature = "alloc")))]
#[derive(Debug, Clone)]
pub struct Req(pub(crate) VecReq);

impl Req {
  /// Creates a new GET request
  pub fn get<P: AsRef<str>>(path: P) -> Self {
    Self(VecReq::get(path))
  }

  /// Creates a new POST request
  pub fn post<P: AsRef<str>>(path: P) -> Self {
    Self(VecReq::post(path))
  }

  /// Creates a new PUT request
  pub fn put<P: AsRef<str>>(path: P) -> Self {
    Self(VecReq::put(path))
  }

  /// Creates a new DELETE request
  pub fn delete<P: AsRef<str>>(path: P) -> Self {
    Self(VecReq::delete(path))
  }
}

impl Deref for Req {
  type Target = VecReq;
  fn deref(&self) -> &VecReq {
    &self.0
  }
}

impl DerefMut for Req {
  fn deref_mut(&mut self) -> &mut VecReq {
    &mut self.0
  }
}

#[cfg(feature = "alloc")]
type VecOpt = Opt<Vec<u8>>;

#[cfg(feature = "alloc")]
type VecReq = ReqCore<Vec<u8>, Vec<u8>, Vec<VecOpt>, Vec<(OptNumber, VecOpt)>>;

/// TODO
#[derive(Debug, Clone)]
pub struct ReqCore<Bytes: Array<Item = u8>,
 OptBytes: Array<Item = u8> + 'static,
 Opts: Array<Item = Opt<OptBytes>>,
 OptNumbers: Array<Item = (OptNumber, Opt<OptBytes>)>>
{
  msg: Message<Bytes, OptBytes, Opts>,
  opts: Option<OptNumbers>,
}

impl<Bytes: Array<Item = u8>,
      OptBytes: Array<Item = u8> + 'static,
      Opts: Array<Item = Opt<OptBytes>>,
      OptNumbers: Array<Item = (OptNumber, Opt<OptBytes>)>> ReqCore<Bytes, OptBytes, Opts, OptNumbers>
{
  fn new<P: AsRef<str>>(method: Method, path: P) -> Self {
    let msg = Message { ty: Type::Con,
                        ver: Default::default(),
                        code: method.0,
                        id: crate::generate_id(),
                        opts: Default::default(),
                        payload: Payload(Default::default()),
                        token: Token(Default::default()),
                        };

    let mut me = Self { msg,
                        opts: Default::default() };

    // Uri-Path
    me.set_option(11, path.as_ref().as_bytes().iter().copied());

    me
  }

  /// Add a custom option to this request
  ///
  /// If there was no room in the collection, returns the arguments back as `Some(number, value)`.
  /// Otherwise, returns `None`.
  pub fn set_option<V: IntoIterator<Item = u8>>(&mut self, number: u32, value: V) -> Option<(u32, V)> {
    if self.opts.is_none() {self.opts = Some(Default::default());}
    crate::add_option(self.opts.as_mut().unwrap(), number, value)
  }

  /// Creates a new GET request
  pub fn get<P: AsRef<str>>(path: P) -> Self {
    Self::new(Method::GET, path)
  }

  /// Creates a new POST request
  pub fn post<P: AsRef<str>>(path: P) -> Self {
    Self::new(Method::POST, path)
  }

  /// Creates a new PUT request
  pub fn put<P: AsRef<str>>(path: P) -> Self {
    Self::new(Method::PUT, path)
  }

  /// Creates a new DELETE request
  pub fn delete<P: AsRef<str>>(path: P) -> Self {
    Self::new(Method::DELETE, path)
  }

  /// Add a payload to this request
  pub fn set_payload<P: IntoIterator<Item = u8>>(&mut self, payload: P) {
    self.msg.payload = Payload(payload.into_iter().collect());
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

  /// Drains the internal associated list of opt number <> opt and converts the numbers into deltas to prepare for message transmission
  fn normalize_opts(&mut self) {
    if let Some(opts) = Option::take(&mut self.opts) {
      self.msg.opts = crate::normalize_opts(opts);
    }
  }
}

impl<Bytes: Array<Item = u8>,
      OptBytes: Array<Item = u8> + 'static,
      Opts: Array<Item = Opt<OptBytes>>,
      OptNumbers: Array<Item = (OptNumber, Opt<OptBytes>)>> From<ReqCore<Bytes, OptBytes, Opts, OptNumbers>>
  for Message<Bytes, OptBytes, Opts>
{
  fn from(mut req: ReqCore<Bytes, OptBytes, Opts, OptNumbers>) -> Self {
    req.normalize_opts();
    req.msg
  }
}
