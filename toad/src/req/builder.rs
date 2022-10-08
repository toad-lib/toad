use no_std_net::SocketAddr;
use toad_common::prelude::*;

use super::{Method, Req};
use crate::option::common_options;
use crate::platform::Platform;
use crate::ToCoapValue;

/// Errors encounterable while using ReqBuilder
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
  /// Ran out of storage space for options
  TooManyOptions,
}

/// Build a request
///
/// note: this is highly experimental and will likely move and change roles. Do not use.
///
/// ```
/// use toad::platform::Std;
/// use toad::req::ReqBuilder;
/// use toad::ContentFormat;
///
/// let payload = r#"""{
///              "name": "Jameson",
///              "say": "Hello"
///            }"""#;
///
/// let request =
///   ReqBuilder::<Std>::get("127.0.0.1:1234".parse().unwrap(), "say_stuff").accept(ContentFormat::Json)
///                                                         .content_format(ContentFormat::Json)
///                                                         .payload(payload)
///                                                         .build()
///                                                         .unwrap();
///
/// let rep = send(&request);
/// assert_eq!(rep.payload_string().unwrap(), "Hello, Jameson!");
/// # fn send(req: &toad::req::Req<Std>) -> toad::resp::Resp<Std> {
/// #   let mut rep = toad::resp::Resp::for_request(req).unwrap();
/// #   rep.set_payload("Hello, Jameson!".bytes());
/// #   rep
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct ReqBuilder<P: Platform> {
  inner: Result<Req<P>, Error>,
}

impl<P: Platform> ReqBuilder<P> {
  fn new(method: Method, host: SocketAddr, path: impl AsRef<str>) -> Self {
    Self { inner: Ok(Req::new(method, host, path)) }
  }

  /// Creates a GET request
  pub fn get(host: SocketAddr, path: impl AsRef<str>) -> Self {
    Self::new(Method::GET, host, path)
  }

  /// Creates a PUT request
  pub fn put(host: SocketAddr, path: impl AsRef<str>) -> Self {
    Self::new(Method::PUT, host, path)
  }

  /// Creates a POST request
  pub fn post(host: SocketAddr, path: impl AsRef<str>) -> Self {
    Self::new(Method::POST, host, path)
  }

  /// Creates a DELETE request
  pub fn delete(host: SocketAddr, path: impl AsRef<str>) -> Self {
    Self::new(Method::DELETE, host, path)
  }

  /// Insert or update an option value - use this for non-Repeatable Options.
  ///
  /// # Errors
  /// Causes the builder to error if the capacity of the options collection is exhausted.
  pub fn option<V: ToCoapValue>(mut self, number: u32, value: V) -> Self {
    self.inner
        .as_mut()
        .map(|inner| inner.set_option(number, value.to_coap_value::<P::MessageOptionBytes>()))
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
        .map(|inner| inner.add_option(number, value.to_coap_value::<P::MessageOptionBytes>()))
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
    self.inner
        .as_mut()
        .perform_mut(|i| i.set_payload(value))
        .ok();
    self
  }

  /// Unwrap the builder into the built request
  pub fn build(self) -> Result<Req<P>, Error> {
    self.inner
  }

  common_options!(P);
}
