use crate::config::Config;
#[cfg(feature = "std")]
use crate::config::Std;
use crate::core::Core;
use crate::req::{Req, ReqBuilder};
use crate::resp::Resp;
use crate::result_ext::{MapErrInto, ResultExt};

/// Config struct containing things needed to make a new Client.
///
/// This is used for bring-your-own platform use cases, like embedded.
#[derive(Clone, Debug)]
pub struct ClientConfig<Cfg: Config> {
  /// The clock that the kwap runtime will use
  /// to keep track of time.
  ///
  /// For `std` platforms, this is [`crate::std::Clock`].
  pub clock: Cfg::Clock,
  /// The network abstraction that the kwap runtime
  /// will use to interact with the network.
  ///
  /// For `std` platforms, this is [`std::net::UdpSocket`].
  pub sock: Cfg::Socket,
}

/// A blocking CoAP request client
#[allow(missing_debug_implementations)]
pub struct Client<Cfg: Config> {
  // TODO: wrap with refcell on non-std or mutex on std
  core: Core<Cfg>,
}

/// Result of fallible Client operation
///
/// `core::result::Result<T, kwap::blocking::client::Error>`
pub type Result<T> = core::result::Result<T, Error>;

/// Helper methods on Client Results
pub trait ClientResultExt<T> {
  /// If we timed out waiting for a response, consider that Ok(None).
  ///
  /// Usually used to handle sending non-confirmable requests that
  /// the server may have received but not responded to.
  fn timeout_ok(self) -> Result<Option<T>>;
}

impl<T> ClientResultExt<T> for Result<T> {
  fn timeout_ok(self) -> Result<Option<T>> {
    match self {
      | Ok(t) => Ok(Some(t)),
      | Err(Error::TimedOut) => Ok(None),
      | Err(e) => Err(e),
    }
  }
}

/// Errors that could be encountered when sending requests or receiving responses.
#[derive(Copy, Clone, Debug)]
pub enum Error {
  /// There was an issue along the network somewhere
  NetworkError,
  /// A message we tried to send was invalid
  MessageInvalid,
  /// We timed out waiting for our request to be sent, or for a response to a request.
  TimedOut,
  /// The host you provided is not a valid utf8 string
  HostInvalidUtf8(core::str::Utf8Error),
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
      | HostInvalidUtf8(e) => Self::HostInvalidUtf8(*e),
      | HostInvalidIpAddress => Self::HostInvalidIpAddress,
      | ClockError => Self::Other,
    }
  }
}

#[cfg(feature = "std")]
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
  pub fn new_std() -> Self {
    let clock = crate::std::Clock::new();
    let sock = std::net::UdpSocket::bind("127.0.0.1:4812").unwrap();
    Client::<Std>::new(ClientConfig { clock, sock })
  }
}

impl<Cfg: Config> Client<Cfg> {
  /// Create a new request client
  pub fn new(ClientConfig { clock, sock }: ClientConfig<Cfg>) -> Self {
    Self {
      core: Core::new(clock, sock),
    }
  }

  /// Ping an endpoint
  ///
  /// Note: this will eventually not require Client to be borrowed mutably.
  pub fn ping(&mut self, host: impl AsRef<str>, port: u16) -> Result<()> {
    self
      .core
      .ping(host, port)
      .map_err_into()
      .bind(|(id, addr)| nb::block!(self.core.poll_ping(id, addr)).map_err_into())
  }

  /// Send a request
  ///
  /// Note: this will eventually not require Client to be borrowed mutably.
  pub fn send(&mut self, req: Req<Cfg>) -> Result<Resp<Cfg>> {
    self
      .core
      .send_req(req)
      .map_err_into()
      .bind(|(token, addr)| nb::block!(self.core.poll_resp(token, addr)).map_err_into())
  }

  /// Send a GET request
  pub fn get(host: impl AsRef<str>, port: u16, path: impl AsRef<str>) -> ReqBuilder<Cfg> {
    ReqBuilder::get(host, port, path)
  }
}
