use kwap_common::Array;

#[cfg(feature = "std")]
use crate::config::Std;
use crate::config::{self, Config};
use crate::core::Core;
use crate::req::Req;
use crate::socket::Addressed;

/// Type alias for a server middleware function. See [`Server.middleware`]
pub type Middleware<Cfg> = dyn Fn(Addressed<Req<Cfg>>) -> (Continue, Action<Cfg>);

/// Should the middleware chain stop or continue processing this message?
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Continue {
  /// This message should continue to be processed by the next middleware.
  Yes,
  /// This message has been fully handled and should not be processed
  /// by any other middleware.
  No,
}

/// Action to perform as a result of middleware
#[derive(Clone, Debug)]
pub enum Action<Cfg: Config> {
  /// Send a message
  Send(Addressed<config::Message<Cfg>>),
  /// Do nothing
  Nop,
}

/// A barebones CoAP server.
///
/// See the documentation for [`Server.try_new`] for example usage.
#[allow(missing_debug_implementations)]
pub struct Server<'a, Cfg: Config, Middlewares: Array<Item = &'a Middleware<Cfg>>> {
  core: Core<Cfg>,
  middleware: Middlewares,
}

#[cfg(feature = "std")]
impl<'a> Server<'a, Std, Vec<&'a Middleware<Std>>> {
  /// Create a new Server
  ///
  /// ```no_run
  /// use kwap::blocking::server::{Action, Continue, Server};
  /// use kwap::config::{Message, Std};
  /// use kwap::req::Req;
  /// use kwap::resp::{code, Resp};
  /// use kwap::socket::Addressed;
  /// use kwap::ContentFormat;
  ///
  /// fn hello(req: Addressed<Req<Std>>) -> (Continue, Action<Std>) {
  ///   match req.data().path() {
  ///     | Ok(Some("hello")) => {
  ///       let mut resp = Resp::for_request(req.data().clone());
  ///
  ///       resp.set_code(code::CONTENT);
  ///       resp.set_option(12, ContentFormat::Json.bytes());
  ///       resp.set_payload(r#"{ "hello": "world" }"#.bytes());
  ///
  ///       let msg = req.as_ref().map(|_| Message::<Std>::from(resp));
  ///
  ///       (Continue::No, Action::Send(msg))
  ///     },
  ///     | _ => (Continue::Yes, Action::Nop),
  ///   }
  /// }
  ///
  /// fn not_found(req: Addressed<Req<Std>>) -> (Continue, Action<Std>) {
  ///   let mut resp = Resp::for_request(req.data().clone());
  ///   resp.set_code(code::NOT_FOUND);
  ///   let msg: Addressed<Message<Std>> = req.as_ref().map(|_| resp.into());
  ///   (Continue::No, Action::Send(msg))
  /// }
  ///
  /// let mut server = Server::<Std, Vec<_>>::try_new([127, 0, 0, 1], 3030).unwrap();
  /// server.middleware(&hello);
  /// server.middleware(&not_found);
  /// ```
  pub fn try_new(ip: [u8; 4], port: u16) -> Result<Self, std::io::Error> {
    let [a, b, c, d] = ip;
    let ip = std::net::Ipv4Addr::new(a, b, c, d);

    std::net::UdpSocket::bind((ip, port)).map(|sock| Self::new(sock, crate::std::Clock::new()))
  }
}

impl<'a, Cfg: Config, Middlewares: Array<Item = &'a Middleware<Cfg>>> Server<'a, Cfg, Middlewares> {
  /// Construct a new Server for the current platform.
  ///
  /// If the standard library is available, see [`Server.try_new`].
  pub fn new(sock: Cfg::Socket, clock: Cfg::Clock) -> Self {
    let core = Core::<Cfg>::new(clock, sock);

    Self { core,
           middleware: Default::default() }
  }

  /// Add a function that will be called with incoming messages.
  ///
  /// These functions, "middleware," perform [`Action`]s and indicate
  /// whether the message should [`Continue`] to be processed by the next
  /// middleware function or not.
  ///
  /// Middleware functions are called in the order that they were registered.
  ///
  /// ```compile_fail
  /// fn hello(Addressed<Req<Cfg>>) -> (Continue, Action) {
  ///   /*
  ///     path == "hello"
  ///     ? (Continue::No, Action::Send(2.05 CONTENT))
  ///     : (Continue::Yes, Action::Nop)
  ///   */
  /// }
  ///
  /// fn not_found(Addressed<Req<Cfg>>) -> (Continue, Action) {
  ///   // always returns (Continue::No, Send(4.04 NOT FOUND))
  /// }
  ///
  /// // This will try to respond to /hello, and if the hello middleware
  /// // fails to process the request then we respond 4.04
  /// server.middleware(hello);
  /// server.middleware(not_found);
  ///
  /// // GOTCHA! This will always respond 4.04
  /// server.middleware(not_found);
  /// server.middleware(hello);
  /// ```
  pub fn middleware(&mut self, f: &'a Middleware<Cfg>) -> () {
    self.middleware.push(f);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  type TestServer<'a> = Server<'a, crate::test::Config, Vec<&'a Middleware<crate::test::Config>>>;

  #[test]
  fn new_server() {
    let sock = crate::test::SockMock::new();
    let clock = crate::test::ClockMock::new();
    TestServer::new(sock, clock);
  }
}
