use crate::{config::Config, ToOptionValue, option::common_options};

use super::{Req, Method};

/// TODO
#[derive(Clone, Debug)]
pub struct ReqBuilder<Cfg: Config> {
  inner: Req<Cfg>,
}

impl<Cfg: Config> ReqBuilder<Cfg> {
  fn new(method: Method, host: impl AsRef<str>, port: u16, path: impl AsRef<str>) -> Self {
    Self {inner: Req::new(method, host, port, path)}
  }

  pub fn get(host: impl AsRef<str>, port: u16, path: impl AsRef<str>) -> Self {
    Self::new(Method::GET, host, port, path)
  }

  pub fn option<V: ToOptionValue>(mut self, number: u32, value: V) -> Self {
    self.inner.set_option(number, value.to_option_value::<Cfg>()).unwrap();
    self
  }

  common_options!(Cfg);
}
