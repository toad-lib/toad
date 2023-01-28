use no_std_net::SocketAddr;
use toad_common::*;
use toad_msg::{OptNumber, OptValue, OptionMap, SetOptionError};

use super::{Method, Req};
use crate::option::common_options;
use crate::platform::{self, PlatformTypes};
use crate::ToCoapValue;

/// Errors encounterable while using ReqBuilder
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error<P>
  where P: PlatformTypes,
        platform::toad_msg::opt::OptValue<P>: Clone + Eq + core::fmt::Debug,
        platform::toad_msg::opt::SetError<P>: Clone + core::fmt::Debug + Eq
{
  /// Ran out of storage space for options
  SetOptionError(platform::toad_msg::opt::SetError<P>),

  /// You tried to set multiple values for a non-repeatable option
  #[allow(missing_docs)]
  OptionNotRepeatable {
    number: OptNumber,
    old: platform::toad_msg::opt::OptValue<P>,
    new: platform::toad_msg::opt::OptValue<P>,
  },
}

/// Build a request
///
/// note: this is highly experimental and will likely move and change roles. Do not use.
///
/// ```
/// use toad::req::ReqBuilder;
/// use toad::std::{dtls, PlatformTypes as Std};
/// use toad::ContentFormat;
///
/// let payload = r#"""{
///              "name": "Jameson",
///              "say": "Hello"
///            }"""#;
///
/// let request = ReqBuilder::<Std<dtls::Y>>::get("say_stuff").accept(ContentFormat::Json)
///                                                           .content_format(ContentFormat::Json)
///                                                           .payload(payload)
///                                                           .build()
///                                                           .unwrap();
///
/// let rep = send(&request);
/// assert_eq!(rep.payload_string().unwrap(), "Hello, Jameson!");
/// # fn send(req: &toad::req::Req<Std<dtls::Y>>) -> toad::resp::Resp<Std<dtls::Y>> {
/// #   let mut rep = toad::resp::Resp::for_request(req).unwrap();
/// #   rep.set_payload("Hello, Jameson!".bytes());
/// #   rep
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct ReqBuilder<P>
  where P: PlatformTypes,
        platform::toad_msg::opt::OptValue<P>: Clone + Eq + core::fmt::Debug,
        platform::toad_msg::opt::SetError<P>: Clone + core::fmt::Debug + Eq
{
  inner: Result<Req<P>, Error<P>>,
}

impl<P> ReqBuilder<P>
  where P: PlatformTypes,
        platform::toad_msg::opt::OptValue<P>: Clone + Eq + core::fmt::Debug,
        platform::toad_msg::opt::SetError<P>: Clone + core::fmt::Debug + Eq
{
  fn new(method: Method, path: impl AsRef<str>) -> Self {
    Self { inner: Ok(Req::new(method, path)) }
  }

  /// Creates a GET request
  pub fn get(path: impl AsRef<str>) -> Self {
    Self::new(Method::GET, path)
  }

  /// Creates a PUT request
  pub fn put(path: impl AsRef<str>) -> Self {
    Self::new(Method::PUT, path)
  }

  /// Creates a POST request
  pub fn post(path: impl AsRef<str>) -> Self {
    Self::new(Method::POST, path)
  }

  /// Creates a DELETE request
  pub fn delete(path: impl AsRef<str>) -> Self {
    Self::new(Method::DELETE, path)
  }

  /// Set the value of a non-repeatable option.
  ///
  /// If the option has already been set, this will yield `Err(Error::OptionNotRepeatable)`.
  pub fn option<V: ToCoapValue>(mut self, number: OptNumber, value: V) -> Self {
    self.inner =
      self.inner.and_then(|mut req| {
                  let val = OptValue(value.to_coap_value::<platform::toad_msg::opt::Bytes<P>>());
                  match req.as_mut().remove(number) {
                    | Some(existing) => {
                      Err(Error::OptionNotRepeatable { number,
                                                       old: existing.into_iter().next().unwrap(),
                                                       new: val })
                    },
                    | None => req.set(number, val)
                                 .map_err(Error::SetOptionError)
                                 .map(|_| req),
                  }
                });

    self
  }

  ///
  pub fn add_option<V: ToCoapValue>(mut self, number: OptNumber, value: V) -> Self {
    self.inner = self.inner.and_then(|mut req| {
                             let val =
                               OptValue(value.to_coap_value::<platform::toad_msg::opt::Bytes<P>>());
                             req.set(number, val)
                                .map_err(Error::SetOptionError)
                                .map(|_| req)
                           });

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
  pub fn build(self) -> Result<Req<P>, Error<P>> {
    self.inner
  }

  common_options!(P);
}
