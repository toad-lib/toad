use embedded_time::duration::Milliseconds;
use embedded_time::Clock;
use kwap_common::prelude::*;
use no_std_net::SocketAddr;

use crate::config::Config;
use crate::core::{Core, Error, Secure, What, When};
use crate::net::{Addrd, Socket};
use crate::platform::Platform;
#[cfg(feature = "std")]
use crate::platform::{Std, StdSecure};
use crate::req::{Req, ReqBuilder};
use crate::resp::Resp;
use crate::std::{secure, SecureUdpSocket};

/// Platform struct containing things needed to make a new Client.
///
/// This is used for bring-your-own platform use cases, like embedded.
#[derive(Clone, Debug)]
pub struct ClientConfig<Cfg: Platform> {
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

// TODO(#80): Make clients usable by multiple threads
// (Send + methods ask for &self and not &mut self)
/// A blocking CoAP request client
#[allow(missing_debug_implementations)]
pub struct Client<Cfg: Platform> {
  core: Core<Cfg>,
}

/// Helper methods on Client Results
pub trait ClientResultExt<T, Cfg: Platform> {
  /// If we timed out waiting for a response, consider that Ok(None).
  ///
  /// Usually used to handle sending non-confirmable requests that
  /// the server may have received but not responded to.
  fn timeout_ok(self) -> Result<Option<T>, Error<Cfg>>;
}

impl<T, Cfg: Platform> ClientResultExt<T, Cfg> for Result<T, Error<Cfg>> {
  fn timeout_ok(self) -> Result<Option<T>, Error<Cfg>> {
    match self {
      | Ok(t) => Ok(Some(t)),
      | Err(Error { what: What::MessageNeverAcked,
                    .. }) => Ok(None),
      | Err(e) => Err(e),
    }
  }
}

#[cfg(feature = "std")]
impl Client<StdSecure> {
  /// Create a new Client secured by DTLS
  ///
  /// ```no_run
  /// use kwap::blocking::Client;
  /// use kwap::req::ReqBuilder;
  /// use kwap::ContentFormat;
  ///
  /// let mut client = Client::new_secure(1234).unwrap();
  /// let req = ReqBuilder::get("127.0.0.1:5683".parse().unwrap(), "hello").accept(ContentFormat::Text)
  ///                                                                      .build()
  ///                                                                      .unwrap();
  ///
  /// let rep = client.send(req).unwrap();
  ///
  /// println!("Hello, {}!", rep.payload_string().unwrap());
  /// ```
  pub fn new_secure(port: u16) -> secure::Result<Self> {
    Client::<StdSecure>::new_secure_config(port, Config::default())
  }

  /// Create a new std client with a specific runtime config
  pub fn new_secure_config(port: u16, config: Config) -> secure::Result<Self> {
    let clock = crate::std::Clock::new();
    std::net::UdpSocket::bind(format!("0.0.0.0:{}", port)).map_err(secure::Error::from).bind(
        SecureUdpSocket::try_new_client
    ).map(|sock|
    Client::<StdSecure>::new_config(config, ClientConfig { clock, sock }))
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
  /// let mut client = Client::new_std();
  /// let req = ReqBuilder::get("127.0.0.1:5683".parse().unwrap(), "hello").accept(ContentFormat::Text)
  ///                                                                      .build()
  ///                                                                      .unwrap();
  ///
  /// let rep = client.send(req).unwrap();
  ///
  /// println!("Hello, {}!", rep.payload_string().unwrap());
  /// ```
  pub fn new_std() -> Self {
    Client::<Std>::new_std_config(Config::default())
  }

  /// Create a new std client with a specific runtime config
  pub fn new_std_config(config: Config) -> Self {
    let clock = crate::std::Clock::new();
    let sock = std::net::UdpSocket::bind("0.0.0.0:1111").unwrap();
    Client::<Std>::new_config(config, ClientConfig { clock, sock })
  }
}

impl<P: Platform> Client<P> {
  /// Create a new request client
  pub fn new(ClientConfig { clock, sock }: ClientConfig<P>) -> Self {
    Self { core: Core::new(clock, sock) }
  }

  /// Create a new request client with a specific runtime config
  pub fn new_config(config: Config, ClientConfig { clock, sock }: ClientConfig<P>) -> Self {
    Self { core: Core::new_config(config, clock, sock) }
  }

  /// Ping an endpoint
  ///
  /// Note: this will eventually not require Client to be borrowed mutably.
  pub fn ping(&mut self, host: impl AsRef<str>, port: u16) -> Result<(), Error<P>> {
    self.core
        .ping(host, port)
        .bind(|(id, addr)| nb::block!(self.core.poll_ping(id, addr)))
  }

  /// Send a request
  ///
  /// Note: this will eventually not require Client to be borrowed mutably.
  pub fn send(&mut self, req: Req<P>) -> Result<Resp<P>, Error<P>> {
    self.core
        .send_req(req, Secure::IfSupported)
        .bind(|(token, addr)| nb::block!(self.core.poll_resp(token, addr)))
  }

  /// Listen on a multicast address for a broadcast from a Server
  ///
  /// This will time out if nothing has been received after 1 second.
  pub fn listen_multicast(clock: P::Clock, port: u16) -> Result<Addrd<Req<P>>, Error<P>> {
    let addr = crate::multicast::all_coap_devices(port);

    P::Socket::bind(addr).map_err(|e| When::None.what(What::SockError(e)))
                         .map(|sock| Self::new(ClientConfig { clock, sock }))
                         .bind(|mut client| loop {
                           let start = client.core.clock.try_now().unwrap();
                           let since_start = |clock: &P::Clock| {
                             let now = clock.try_now().unwrap();
                             Milliseconds::<u64>::try_from(now - start).unwrap()
                           };

                           match client.core.poll_req() {
                             | Err(nb::Error::Other(e)) => {
                               log::error!("{:?}", e);
                               break Err(e);
                             },
                             | Err(nb::Error::WouldBlock)
                               if since_start(&client.core.clock) > Milliseconds(1000u64) =>
                             {
                               log::error!("timeout");
                               break Err(When::None.what(What::Timeout));
                             },
                             | Err(nb::Error::WouldBlock) => (),
                             | Ok(x) => break Ok(x),
                           }
                         })
  }

  /// Send a GET request
  pub fn get(host: SocketAddr, path: impl AsRef<str>) -> ReqBuilder<P> {
    ReqBuilder::get(host, path)
  }
}
