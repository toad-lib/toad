#![allow(missing_docs)]

use crate::config::{Config, Std};
use crate::core::Core;
use crate::req::{Req, ReqBuilder};
use crate::resp::Resp;
use crate::result_ext::{MapErrInto, ResultExt};

/// TODO
#[allow(missing_debug_implementations)]
pub struct Client<Cfg: Config> {
  // TODO: wrap with refcell on non-std or mutex on std
  core: Core<Cfg>,
}

pub type Result<T> = core::result::Result<T, Error>;

/// Client error
#[derive(Copy, Clone, Debug)]
pub enum Error {
  NetworkError,
  MessageInvalid,
  TimedOut,
  HostInvalidUtf8,
  HostInvalidIpAddress,
  Other,
}

impl<Cfg: Config> From<crate::core::Error<Cfg>> for Error {
  fn from(e: crate::core::Error<Cfg>) -> Self {
    Self::from(&e)
  }
}

impl<'a, Cfg: Config> From<&'a crate::core::Error<Cfg>> for Error {
  fn from(e: &'a crate::core::Error<Cfg>) -> Self {
    use crate::core::Error::*;
    match e {
      | SockError(_) => Self::NetworkError,
      | ToBytes(_) => Self::MessageInvalid,
      | MessageNeverAcked => Self::TimedOut,
      | HostInvalidUtf8(_) => Self::HostInvalidUtf8,
      | HostInvalidIpAddress => Self::HostInvalidIpAddress,
      | ClockError => Self::Other,
    }
  }
}

#[cfg(not(feature = "no_std"))]
impl Client<Std> {
  /// TODO
  ///
  /// ```ignore
  /// use kwap::blocking::Client;
  ///
  /// fn main() {
  ///   let client = Client::new_std();
  ///   let rep = client.get("127.0.0.1", 5683, "hello")
  ///                   .allow(ContentFormat::Text)
  ///                   .unwrap()
  ///                   .send()
  ///                   .unwrap()
  ///                   .ensure_ok()
  ///                   .unwrap();
  ///
  ///   println!("Hello, {}!", rep.payload_string());
  /// }
  /// ```
  pub fn new_std() -> Client<Std> {
    let clock = crate::std::Clock::new();
    let sock = std::net::UdpSocket::bind("127.0.0.1:4812").unwrap();
    Client { core: Core::new(clock, sock) }
  }
}

impl<Cfg: Config> Client<Cfg> {
  /// Ping an endpoint
  pub fn ping(&mut self, host: impl AsRef<str>, port: u16) -> Result<()> {
    self.core
        .ping(host, port)
        .map_err_into()
        .bind(|(id, addr)| nb::block!(self.core.poll_ping(id, addr)).map_err_into())
  }

  /// Send a request
  pub fn send(&mut self, req: Req<Cfg>) -> Result<Resp<Cfg>> {
    self.core
        .send_req(req)
        .map_err_into()
        .bind(|(token, addr)| nb::block!(self.core.poll_resp(token, addr)).map_err_into())
  }

  /// Send a GET request
  pub fn get(host: impl AsRef<str>, port: u16, path: impl AsRef<str>) -> ReqBuilder<Cfg> {
    ReqBuilder::get(host, port, path)
  }
}
