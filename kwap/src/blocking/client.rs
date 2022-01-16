#![allow(missing_docs)]

use crate::{config::{Config, Std}, core::Core, resp::Resp, req::ReqBuilder};

/// TODO
#[allow(missing_debug_implementations)]
pub struct Client<Cfg: Config> {
  core: Core<Cfg>,
}

pub type Result<T> = core::result::Result<T, Error>;

/// Client error
#[derive(Clone, Debug)]
pub enum Error {
  NetworkError,
  MessageInvalid,
  TimedOut,
  HostInvalidUtf8,
  HostInvalidIpAddress,
  Other,
}

impl<'a, Cfg: Config> From<&'a crate::core::Error<Cfg>> for Error {
  fn from(e: &'a crate::core::Error<Cfg>) -> Self {
    use crate::core::Error::*;
    match e {
      SockError(_) => Self::NetworkError,
      ToBytes(_) => Self::MessageInvalid,
      MessageNeverAcked => Self::TimedOut,
      HostInvalidUtf8(_) => Self::HostInvalidUtf8,
      HostInvalidIpAddress => Self::HostInvalidIpAddress,
      ClockError => Self::Other,
    }
  }
}

impl<Cfg: Config> Client<Cfg> {
  /// TODO
  ///
  /// ```no_run
  /// use kwap::blocking::Client;
  ///
  /// fn main() {
  ///   let client = Client::new_std();
  ///   let rep = client.get("127.0.0.1", 5683, "hello")?.ensure_ok()?;
  ///   println!("Hello, {}!", rep.payload_string());
  /// }
  /// ```
  #[cfg(not(feature = "no_std"))]
  pub fn new_std() -> Client<Std> {
    let clock = crate::std::Clock::new();
    let sock = std::net::UdpSocket::bind("127.0.0.1:*").unwrap();
    Client {core: Core::new(clock, sock)}
  }

  /// Send a GET request
  pub fn get(host: impl AsRef<str>, port: u16, path: impl AsRef<str>) -> ReqBuilder<Cfg> {
    todo!()
  }
}
