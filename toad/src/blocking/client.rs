use embedded_time::duration::Milliseconds;
use embedded_time::Clock;
use no_std_net::SocketAddr;
use toad_common::*;

use crate::config::Config;
use crate::core::{Core, Error, Secure, What, When};
use crate::net::{Addrd, Socket};
use crate::platform::Platform;
#[cfg(feature = "std")]
use crate::platform::{Std, StdSecure};
use crate::req::{ReqBuilder, ReqForPlatform as Req};
use crate::resp::RespForPlatform as Resp;
#[cfg(feature = "std")]
use crate::std::{secure, SecureUdpSocket};
use crate::time::{Millis, Timeout};

/// Platform struct containing things needed to make a new Client.
///
/// This is used for bring-your-own platform use cases, like embedded.
#[derive(Clone, Debug)]
pub struct ClientConfig<Clock, Socket> {
  /// The clock that the toad runtime will use
  /// to keep track of time.
  ///
  /// For `std` platforms, this is [`crate::std::Clock`].
  pub clock: Clock,
  /// The network abstraction that the toad runtime
  /// will use to interact with the network.
  ///
  /// For `std` platforms, this is [`std::net::UdpSocket`].
  pub sock: Socket,
}

// TODO(#80): Make clients usable by multiple threads
// (Send + methods ask for &self and not &mut self)
/// A blocking CoAP request client
#[allow(missing_debug_implementations)]
pub struct Client<P: Platform> {
  core: Core<P>,
}

/// Helper methods on Client Results
pub trait ClientResultExt<T, P: Platform> {
  /// If we timed out waiting for a response, consider that Ok(None).
  ///
  /// Usually used to handle sending non-confirmable requests that
  /// the server may have received but not responded to.
  fn timeout_ok(self) -> Result<Option<T>, Error<P>>;
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
  /// use toad::platform::Std;
  /// use toad::blocking::Client;
  /// use toad::req::ReqBuilder;
  /// use toad::ContentFormat;
  ///
  /// let mut client = Client::try_new_secure(1234).unwrap();
  /// let req = ReqBuilder::<Std>::get("127.0.0.1:5683".parse().unwrap(), "hello").accept(ContentFormat::Text)
  ///                                                                      .build()
  ///                                                                      .unwrap();
  ///
  /// let rep = client.send(req).unwrap();
  ///
  /// println!("Hello, {}!", rep.payload_string().unwrap());
  /// ```
  pub fn try_new_secure(port: u16) -> secure::Result<Self> {
    Client::<StdSecure>::try_new_secure_config(port, Config::default())
  }

  /// Create a new std client with a specific runtime config
  pub fn try_new_secure_config(port: u16, config: Config) -> secure::Result<Self> {
    let clock = crate::std::Clock::new();
    let addr = format!("0.0.0.0:{}", port);

    std::net::UdpSocket::bind(addr).map_err(secure::Error::from)
                                   .bind(SecureUdpSocket::try_new_client)
                                   .map(|sock| {
                                     let client = ClientConfig { clock, sock };
                                     Client::<StdSecure>::new_config(config, client)
                                   })
  }
}

#[cfg(feature = "std")]
impl Client<Std> {
  /// Create a new Client for a platform supporting Rust's standard library.
  ///
  /// ```no_run
  /// use toad::platform::Std;
  /// use toad::blocking::Client;
  /// use toad::req::ReqBuilder;
  /// use toad::ContentFormat;
  ///
  /// let mut client = Client::new_std(1111);
  /// let req = ReqBuilder::<Std>::get("127.0.0.1:5683".parse().unwrap(), "hello").accept(ContentFormat::Text)
  ///                                                                      .build()
  ///                                                                      .unwrap();
  ///
  /// let rep = client.send(req).unwrap();
  ///
  /// println!("Hello, {}!", rep.payload_string().unwrap());
  /// ```
  pub fn new_std(port: u16) -> Self {
    Client::<Std>::new_std_config(port, Config::default())
  }

  /// Create a new std client with a specific runtime config
  pub fn new_std_config(port: u16, config: Config) -> Self {
    let clock = crate::std::Clock::new();
    let addr = format!("0.0.0.0:{}", port);
    let sock = std::net::UdpSocket::bind(addr).unwrap();
    Client::<Std>::new_config(config, ClientConfig { clock, sock })
  }
}

impl<P: Platform> Client<P> {
  /// Create a new request client
  pub fn new(ClientConfig { clock, sock }: ClientConfig<P::Clock, P::Socket>) -> Self {
    Self { core: Core::new(clock, sock) }
  }

  /// Create a new request client with a specific runtime config
  pub fn new_config(config: Config,
                    ClientConfig { clock, sock }: ClientConfig<P::Clock, P::Socket>)
                    -> Self {
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

  /// Listen on a multicast address for a broadcast from a Server.
  pub fn listen_multicast(clock: P::Clock,
                          port: u16,
                          timeout: Timeout)
                          -> Result<Addrd<Req<P>>, Error<P>> {
    let addr = crate::multicast::all_coap_devices(port);

    P::Socket::bind(addr).map_err(|e| When::None.what(What::SockError(e)))
                         .map(|sock| Self::new(ClientConfig { clock, sock }))
                         .bind(|mut client| loop {
                           let start = client.core.clock.try_now().unwrap();
                           let timed_out = |clock: &P::Clock| match timeout {
                             | Timeout::Millis(ms) => {
                               let now = clock.try_now().unwrap();
                               let elapsed = Millis::try_from(now - start).unwrap();
                               elapsed > Milliseconds(ms)
                             },
                             | _ => false,
                           };

                           match client.core.poll_req() {
                             | Ok(x) => break Ok(x),
                             | Err(nb::Error::Other(e)) => {
                               log::error!("{:?}", e);
                               break Err(e);
                             },
                             | Err(nb::Error::WouldBlock) => {
                               if timed_out(&client.core.clock) {
                                 log::error!("ERROR: timed out");
                                 break Err(When::None.what(What::Timeout));
                               }
                             },
                           }
                         })
  }

  /// Send a GET request
  pub fn get(host: SocketAddr, path: impl AsRef<str>) -> ReqBuilder<P> {
    ReqBuilder::get(host, path)
  }
}
