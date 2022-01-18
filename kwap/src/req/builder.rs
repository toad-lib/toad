use super::{Method, Req};
use crate::config::Config;
use crate::option::common_options;
use crate::ToCoapValue;

/// TODO
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
  inner: Req<Cfg>,
}

impl<Cfg: Config> ReqBuilder<Cfg> {
  fn new(method: Method, host: impl AsRef<str>, port: u16, path: impl AsRef<str>) -> Self {
    Self { inner: Req::new(method, host, port, path) }
  }

  pub fn get(host: impl AsRef<str>, port: u16, path: impl AsRef<str>) -> Self {
    Self::new(Method::GET, host, port, path)
  }

  pub fn option<V: ToCoapValue>(mut self, number: u32, value: V) -> Self {
    // TODO: Some handling
    self.inner.set_option(number, value.to_coap_value::<Cfg::OptBytes>());
    self
  }

  pub fn add_option<V: ToCoapValue>(mut self, number: u32, value: V) -> Self {
    // TODO: Some handling
    self.inner.add_option(number, value.to_coap_value::<Cfg::OptBytes>());
    self
  }

  pub fn payload<V: ToCoapValue>(mut self, value: V) -> Self {
    self.inner.set_payload(value);
    self
  }

  pub fn build(self) -> Result<Req<Cfg>, ()> {
    Ok(self.inner)
  }

  common_options!(Cfg);
}
