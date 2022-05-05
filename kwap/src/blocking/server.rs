use kwap_common::Array;

#[cfg(feature = "std")]
use crate::config::Std;
use crate::config::{self, Config};
use crate::core::{Core, Error};
use crate::req::Req;
use crate::socket::Addressed;

/// Data structure used by server for bookkeeping of
/// "the result of the last middleware run and what to do next"
#[derive(Debug)]
enum Status<Cfg: Config> {
  Err(Error<Cfg>),
  Continue,
  Stop,
  Exit,
}

impl<Cfg: Config> Status<Cfg> {
  fn from_continue(cont: Continue) -> Self {
    match cont {
      | Continue::Yes => Self::Continue,
      | Continue::No => Self::Stop,
    }
  }

  fn bind_result(self, result: Result<(), Error<Cfg>>) -> Self {
    match &self {
      | Self::Err(_) => self,
      | Self::Stop | Self::Exit | Self::Continue => {
        result.map(|_| self).map_err(|e| Self::Err(e)).unwrap_or_else(|e| e)
      },
    }
  }
}

/// Type alias for a server middleware function. See [`Server.middleware`]
pub type Middleware<Cfg> = dyn Fn(&Addressed<Req<Cfg>>) -> (Continue, Action<Cfg>);

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
  /// Stop the server completely
  Exit,
  /// Do nothing
  Nop,
}

/// A barebones CoAP server.
///
/// See the documentation for [`Server.try_new`] for example usage.
#[allow(missing_debug_implementations)]
pub struct Server<'a, Cfg: Config, Middlewares: Array<Item = &'a Middleware<Cfg>>> {
  core: Core<Cfg>,
  fns: Middlewares,
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
  /// fn hello(req: &Addressed<Req<Std>>) -> (Continue, Action<Std>) {
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
  /// fn not_found(req: &Addressed<Req<Std>>) -> (Continue, Action<Std>) {
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
           fns: Default::default() }
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
    self.fns.push(f);
  }

  /// Start the server
  pub fn start(&mut self) -> Result<(), Error<Cfg>> {
    loop {
      let req = nb::block!(self.core.poll_req())?;

      let mut use_middleware = |middleware: &&'a Middleware<Cfg>| match middleware(&req) {
        | (cont, Action::Nop) => Status::<Cfg>::from_continue(cont),
        | (cont, Action::Send(msg)) => Status::<Cfg>::from_continue(cont).bind_result(self.core.send_msg(msg)),
        | (_, Action::Exit) => Status::Exit,
      };

      let status = self.fns.iter().fold(Status::Continue, |status, f| match status {
                                    | Status::Exit | Status::Err(_) | Status::Stop => status,
                                    | Status::Continue => use_middleware(f),
                                  });

      match status {
        | Status::Exit => return Ok(()),
        | _ => continue,
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use core::time::Duration;
  use std::ops::Deref;
  use std::thread;

  use kwap_msg::{TryFromBytes, TryIntoBytes};
  use no_std_net::{Ipv4Addr, SocketAddrV4};

  use super::*;
  use crate::resp::{code, Resp};
  use crate::test::{ClockMock, Config as Test, SockMock, Timeout};

  type TestServer<'a> = Server<'a, Test, Vec<&'a Middleware<Test>>>;

  fn not_found(req: &Addressed<Req<Test>>) -> (Continue, Action<Test>) {
    let reply = req.as_ref().map(|req| {
                              let mut resp = Resp::<Test>::for_request(req.clone());
                              resp.set_code(code::NOT_FOUND);
                              resp.into()
                            });

    (Continue::Yes, Action::Send(reply))
  }

  fn exit(_: &Addressed<Req<Test>>) -> (Continue, Action<Test>) {
    (Continue::No, Action::Exit)
  }

  #[test]
  fn test_new() {
    let sock = SockMock::new();
    let clock = ClockMock::new();
    TestServer::new(sock, clock);
  }

  #[test]
  fn server_not_found() {
    let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 1234);
    let sock = SockMock::init(addr.into(), vec![]);
    let clock = ClockMock::new();

    let timeout = Timeout::new(Duration::from_secs(1));

    let cancel_timeout = timeout.eject_canceler();
    let inbound_bytes = sock.rx.clone();
    let outbound_bytes = sock.tx.clone();

    let handle = thread::spawn(move || {
      let mut server = TestServer::new(sock, clock);

      server.middleware(&not_found);
      server.middleware(&exit);
      server.start().unwrap();

      cancel_timeout();

      let outbound_rep = {
        let bytes = outbound_bytes.lock().unwrap();
        let msg = config::Message::<Test>::try_from_bytes(bytes.deref()).unwrap();
        Resp::<Test>::from(msg)
      };

      assert_eq!(outbound_rep.code(), code::NOT_FOUND);
    });

    let req = Req::<Test>::get("0.0.0.0", 1234, "hello");
    let msg: config::Message<Test> = req.into();
    inbound_bytes.lock().unwrap().append(&mut msg.try_into_bytes().unwrap());

    timeout.wait();
    handle.join().unwrap();
  }
}
