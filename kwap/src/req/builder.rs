use super::{Method, Req};
use crate::config::Config;
use crate::option::common_options;
use crate::result_ext::ResultExt;
use crate::ToCoapValue;

/// Errors encounterable while using ReqBuilder
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
  /// Ran out of storage space for options
  TooManyOptions,
}

/// Build a request
///
/// NOTE: this is highly experimental and will likely move and change roles. Do not use.
///
/// ```
/// use kwap::config::Std;
/// use kwap::req::ReqBuilder;
/// use kwap::ContentFormat;
///
/// let payload = r#"""{
///              "name": "Jameson",
///              "say": "Hello"
///            }"""#;
///
/// let request = ReqBuilder::<Std>::get("127.0.0.1", 1234, "say_stuff").accept(ContentFormat::Json)
///                                                                     .content_format(ContentFormat::Json)
///                                                                     .payload(payload)
///                                                                     .build()
///                                                                     .unwrap();
///
/// let rep = send(request);
/// assert_eq!(rep.payload_string().unwrap(), "Hello, Jameson!");
/// # fn send(req: kwap::req::Req<Std>) -> kwap::resp::Resp<Std> {
/// #   let mut rep = kwap::resp::Resp::for_request(req);
/// #   rep.set_payload("Hello, Jameson!".bytes());
/// #   rep
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct ReqBuilder<Cfg: Config> {
  inner: Result<Req<Cfg>, Error>,
}

impl<Cfg: Config> ReqBuilder<Cfg> {
  fn new(method: Method, host: impl AsRef<str>, port: u16, path: impl AsRef<str>) -> Self {
    Self { inner: Ok(Req::new(method, host, port, path)) }
  }

  /// Creates a GET request
  pub fn get(host: impl AsRef<str>, port: u16, path: impl AsRef<str>) -> Self {
    Self::new(Method::GET, host, port, path)
  }
  /// Creates a PUT request
  pub fn put(host: impl AsRef<str>, port: u16, path: impl AsRef<str>) -> Self {
    Self::new(Method::PUT, host, port, path)
  }
  /// Creates a POST request
  pub fn post(host: impl AsRef<str>, port: u16, path: impl AsRef<str>) -> Self {
    Self::new(Method::POST, host, port, path)
  }
  /// Creates a DELETE request
  pub fn delete(host: impl AsRef<str>, port: u16, path: impl AsRef<str>) -> Self {
    Self::new(Method::DELETE, host, port, path)
  }

  /// Insert or update an option value - use this for non-Repeatable Options.
  ///
  /// # Errors
  /// Causes the builder to error if the capacity of the options collection is exhausted.
  pub fn option<V: ToCoapValue>(mut self, number: u32, value: V) -> Self {
    self.inner
        .as_mut()
        .map(|inner| inner.set_option(number, value.to_coap_value::<Cfg::OptBytes>()))
        .map_err(|e| *e)
        .perform(|res| match res {
          | Some(_) => self.inner = Err(Error::TooManyOptions),
          | None => (),
        })
        .ok();

    self
  }

  /// Insert an option value - use this for Repeatable Options.
  ///
  /// # Errors
  /// Causes the builder to error if the capacity of the options collection is exhausted.
  pub fn add_option<V: ToCoapValue>(mut self, number: u32, value: V) -> Self {
    self.inner
        .as_mut()
        .map(|inner| inner.add_option(number, value.to_coap_value::<Cfg::OptBytes>()))
        .map_err(|e| *e)
        .perform(|res| match res {
          | Some(_) => self.inner = Err(Error::TooManyOptions),
          | None => (),
        })
        .ok();

    self
  }

  /// Set the payload of the request
  pub fn payload<V: ToCoapValue>(mut self, value: V) -> Self {
    self.inner.as_mut().perform_mut(|i| i.set_payload(value)).ok();
    self
  }

  /// Unwrap the builder into the built request
  pub fn build(self) -> Result<Req<Cfg>, Error> {
    self.inner
  }

  common_options!(Cfg);
}
