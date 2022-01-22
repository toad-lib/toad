use crate::config::{Config, Std};
use crate::core::Core;
use crate::req::{Req, ReqBuilder};
use crate::resp::Resp;
use crate::result_ext::{MapErrInto, ResultExt};

/// A blocking CoAP request client
#[allow(missing_debug_implementations)]
pub struct Client<Cfg: Config> {
  // TODO: wrap with refcell on non-std or mutex on std
  core: Core<Cfg>,
}

/// Client result
pub type Result<T> = core::result::Result<T, Error>;

/// Client error
#[derive(Copy, Clone, Debug)]
pub enum Error {
  /// There was an issue along the network somewhere
  NetworkError,
  /// A message we tried to send was invalid
  MessageInvalid,
  /// We timed out waiting for our request to be sent, or for a response to a request.
  TimedOut,
  /// The host you provided is not a valid utf8 string
  HostInvalidUtf8,
  /// The host you provided is not a valid ip address
  HostInvalidIpAddress,
  /// Some other error
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
  /// Create a new Client for a platform supporting Rust's standard library.
  ///
  /// ```no_run
  /// use kwap::blocking::Client;
  /// use kwap::req::ReqBuilder;
  /// use kwap::ContentFormat;
  ///
  /// fn main() {
  ///   let mut client = Client::new_std();
  ///   let req = ReqBuilder::get("127.0.0.1", 5683, "hello").accept(ContentFormat::Text)
  ///                                                        .build()
  ///                                                        .unwrap();
  ///
  ///   let rep = client.send(req).unwrap();
  ///
  ///   println!("Hello, {}!", rep.payload_string().unwrap());
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
  ///
  /// Note: this will eventually not require Client to be borrowed mutably.
  pub fn ping(&mut self, host: impl AsRef<str>, port: u16) -> Result<()> {
    self.core
        .ping(host, port)
        .map_err_into()
        .bind(|(id, addr)| nb::block!(self.core.poll_ping(id, addr)).map_err_into())
  }

  /// Send a request
  ///
  /// Note: this will eventually not require Client to be borrowed mutably.
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
