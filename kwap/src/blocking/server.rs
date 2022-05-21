use kwap_common::Array;
use kwap_msg::Type;

use crate::config::Config;
use crate::core::{Core, Error};
use crate::net::Addrd;
#[cfg(feature = "std")]
use crate::platform::Std;
use crate::platform::{self, Platform};
use crate::req::{Method, Req};

/// Data structure used by server for bookkeeping of
/// "the result of the last middleware run and what to do next"
#[derive(Debug)]
enum Status<Cfg: Platform> {
  Err(Error<Cfg>),
  Continue,
  Stop,
  Exit,
}

impl<Cfg: Platform> Status<Cfg> {
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
pub type Middleware<Cfg> = dyn Fn(&Addrd<Req<Cfg>>) -> (Continue, Action<Cfg>);

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
pub enum Action<Cfg: Platform> {
  /// Send a message
  Send(Addrd<platform::Message<Cfg>>),
  /// Stop the server completely
  Exit,
  /// Do nothing
  Nop,
}

/// A barebones CoAP server.
///
/// See the documentation for [`Server.try_new`] for example usage.
// TODO(#85): allow opt-out of always piggybacked ack responses
#[allow(missing_debug_implementations)]
pub struct Server<'a, Cfg: Platform, Middlewares: 'static + Array<Item = &'a Middleware<Cfg>>> {
  core: Core<Cfg>,
  fns: Middlewares,
}

#[cfg(feature = "std")]
impl<'a> Server<'a, Std, Vec<&'a Middleware<Std>>> {
  /// Create a new Server
  ///
  /// ```no_run
  /// use kwap::blocking::server::{Action, Continue, Server};
  /// use kwap::net::Addrd;
  /// use kwap::platform::{Message, Std};
  /// use kwap::req::Req;
  /// use kwap::resp::{code, Resp};
  /// use kwap::ContentFormat;
  ///
  /// fn hello(req: &Addrd<Req<Std>>) -> (Continue, Action<Std>) {
  ///   match req.data().path() {
  ///     | Ok(Some("hello")) => {
  ///       let mut resp = Resp::for_request(req.data()).unwrap();
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
  /// fn not_found(req: &Addrd<Req<Std>>) -> (Continue, Action<Std>) {
  ///   let mut resp = Resp::for_request(req.data()).unwrap();
  ///   resp.set_code(code::NOT_FOUND);
  ///   let msg: Addrd<Message<Std>> = req.as_ref().map(|_| resp.into());
  ///   (Continue::No, Action::Send(msg))
  /// }
  ///
  /// let mut server = Server::<Std, Vec<_>>::try_new([127, 0, 0, 1], 3030).unwrap();
  /// server.middleware(&hello);
  /// server.middleware(&not_found);
  /// ```
  pub fn try_new(ip: [u8; 4], port: u16) -> Result<Self, std::io::Error> {
    Self::try_new_config(Config::default(), ip, port)
  }

  /// Create a new std server with a specific runtime config
  pub fn try_new_config(config: Config, ip: [u8; 4], port: u16) -> Result<Self, std::io::Error> {
    let [a, b, c, d] = ip;
    let ip = std::net::Ipv4Addr::new(a, b, c, d);

    std::net::UdpSocket::bind((ip, port)).map(|sock| Self::new_config(config, sock, crate::std::Clock::new()))
  }
}

impl<'a, Cfg: Platform, Middlewares: 'static + Array<Item = &'a Middleware<Cfg>>> Server<'a, Cfg, Middlewares> {
  /// Construct a new Server for the current platform.
  ///
  /// If the standard library is available, see [`Server.try_new`].
  pub fn new(sock: Cfg::Socket, clock: Cfg::Clock) -> Self {
    Self::new_config(Config::default(), sock, clock)
  }

  /// Create a new server with a specific runtime config
  pub fn new_config(config: Config, sock: Cfg::Socket, clock: Cfg::Clock) -> Self {
    let core = Core::<Cfg>::new_config(config, clock, sock);

    let mut self_ = Self { core,
                           fns: Default::default() };
    self_.middleware(&Self::respond_ping);

    self_
  }

  /// Middleware function that responds to CoAP pings (EMPTY Confirmable messages)
  ///
  /// This is included when Server::new is invoked.
  pub fn respond_ping(req: &Addrd<Req<Cfg>>) -> (Continue, Action<Cfg>) {
    match (req.data().method(), req.data().msg_type()) {
      | (Method::EMPTY, Type::Con) => {
        let resp = platform::Message::<Cfg> { ver: Default::default(),
                                              ty: Type::Reset,
                                              id: req.data().msg_id(),
                                              token: kwap_msg::Token(Default::default()),
                                              code: kwap_msg::Code::new(0, 0),
                                              opts: Default::default(),
                                              payload: kwap_msg::Payload(Default::default()) };

        (Continue::No, Action::Send(req.as_ref().map(|_| resp)))
      },
      | _ => (Continue::Yes, Action::Nop),
    }
  }

  /// Add a function that will be called with incoming messages.
  ///
  /// These functions, "middleware," perform [`Action`]s and indicate
  /// whether the message should [`Continue`] to be processed by the next
  /// middleware function or not.
  ///
  /// Middleware functions are called in the order that they were registered.
  ///
  /// ```ignore
  /// fn hello(Addrd<Req<Cfg>>) -> (Continue, Action) {
  ///   /*
  ///     path == "hello"
  ///     ? (Continue::No, Action::Send(2.05 CONTENT))
  ///     : (Continue::Yes, Action::Nop)
  ///   */
  /// }
  ///
  /// fn not_found(Addrd<Req<Cfg>>) -> (Continue, Action) {
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
  use std::thread;

  use kwap_msg::{Id, Token, Type};
  use no_std_net::{Ipv4Addr, SocketAddr, SocketAddrV4};

  use super::*;
  use crate::req::method::Method;
  use crate::resp::{code, Resp};
  use crate::test::{ClockMock, Config as Test, SockMock, Timeout};

  type TestServer<'a> = Server<'a, Test, Vec<&'a Middleware<Test>>>;

  mod ware {
    use super::*;
    pub fn panics(_: &Addrd<Req<Test>>) -> (Continue, Action<Test>) {
      panic!()
    }

    pub fn not_found(req: &Addrd<Req<Test>>) -> (Continue, Action<Test>) {
      let reply = req.as_ref().map(|req| {
                                let mut resp = Resp::<Test>::for_request(&req).unwrap();
                                resp.set_code(code::NOT_FOUND);
                                resp.into()
                              });

      (Continue::Yes, Action::Send(reply))
    }

    pub fn hello(req: &Addrd<Req<Test>>) -> (Continue, Action<Test>) {
      if req.0.method() == Method::GET && req.0.path().unwrap() == Some("hello") {
        let reply = req.as_ref().map(|req| {
                                  let mut resp = Resp::<Test>::for_request(&req).unwrap();
                                  resp.set_payload("hello!".bytes());
                                  resp.set_code(code::CONTENT);
                                  resp.into()
                                });

        (Continue::No, Action::Send(reply))
      } else {
        (Continue::Yes, Action::Nop)
      }
    }

    pub fn exit(req: &Addrd<Req<Test>>) -> (Continue, Action<Test>) {
      if req.0.path().unwrap() == Some("exit") {
        (Continue::No, Action::Exit)
      } else {
        (Continue::Yes, Action::Nop)
      }
    }
  }

  fn setup(timeout: Duration) -> (ClockMock, Timeout, SockMock, SocketAddr) {
    let clock = ClockMock::new();
    let timeout = Timeout::new(timeout);
    let sock = SockMock::new();
    let addr: SocketAddr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 1234).into();

    (clock, timeout, sock, addr)
  }

  #[test]
  fn new_does_not_panic() {
    let sock = SockMock::new();
    let clock = ClockMock::new();
    let mut srv = TestServer::new(sock, clock);
    srv.middleware(&ware::panics);
  }

  #[test]
  fn exit() {
    let (clock, timeout, sock, addr) = setup(Duration::from_secs(1));
    let inbound_bytes = sock.rx.clone();
    let timeout_state = timeout.state.clone();

    let mut say_exit = Req::<Test>::get("0.0.0.0", 1234, "exit");
    say_exit.set_msg_token(Token(Default::default()));
    say_exit.set_msg_id(Id(1));

    SockMock::send_msg::<Test>(&inbound_bytes, Addrd(say_exit.into(), addr));

    let server = thread::spawn(move || {
      let mut server = TestServer::new(sock, clock);

      server.middleware(&ware::exit);
      server.start().unwrap();

      Timeout::cancel(timeout_state);
    });

    timeout.wait();
    server.join().unwrap();
  }

  #[test]
  fn not_found_fallback() {
    let (clock, timeout, sock, addr) = setup(Duration::from_secs(1));
    let (inbound_bytes, outbound_bytes) = (sock.rx.clone(), sock.tx.clone());
    let timeout_state = timeout.state.clone();

    let mut say_hello = Req::<Test>::get("0.0.0.0", 1234, "hello");
    say_hello.set_msg_token(Token(Default::default()));
    say_hello.set_msg_id(Id(1));

    let mut say_exit = Req::<Test>::get("0.0.0.0", 1234, "exit");
    say_exit.set_msg_token(Token(Default::default()));
    say_exit.set_msg_id(Id(2));

    let server = thread::spawn(move || {
      let mut server = TestServer::new(sock, clock);

      server.middleware(&ware::not_found);
      server.middleware(&ware::exit);
      server.start().unwrap();

      Timeout::cancel(timeout_state);
    });

    let work = thread::spawn(move || {
      SockMock::send_msg::<Test>(&inbound_bytes, Addrd(say_hello.into(), addr));

      let rep: Resp<Test> = SockMock::await_msg::<Test>(addr, &outbound_bytes).into();

      assert_eq!(rep.code(), code::NOT_FOUND);
      assert_eq!(rep.msg_type(), kwap_msg::Type::Ack);

      SockMock::send_msg::<Test>(&inbound_bytes, Addrd(say_exit.into(), addr));
    });

    timeout.wait();
    work.join().unwrap();
    server.join().unwrap();
  }

  #[test]
  fn ping() {
    let (clock, timeout, sock, addr) = setup(Duration::from_secs(1));
    let (inbound_bytes, outbound_bytes) = (sock.rx.clone(), sock.tx.clone());
    let timeout_state = timeout.state.clone();

    let mut ping = Req::<Test>::new(Method::EMPTY, "0.0.0.0", 1234, "");
    ping.set_msg_token(Token(Default::default()));
    ping.set_msg_id(Id(1));
    let mut say_exit = Req::<Test>::get("0.0.0.0", 1234, "exit");
    say_exit.set_msg_token(Token(Default::default()));
    say_exit.set_msg_id(Id(2));

    let server = thread::spawn(move || {
      let mut server = TestServer::new(sock, clock);

      server.middleware(&ware::exit);
      server.start().unwrap();

      Timeout::cancel(timeout_state);
    });

    let work = thread::spawn(move || {
      SockMock::send_msg::<Test>(&inbound_bytes, Addrd(ping.into(), addr));

      let rep: Resp<Test> = SockMock::await_msg::<Test>(addr, &outbound_bytes).into();

      assert_eq!(rep.msg_type(), Type::Reset);

      SockMock::send_msg::<Test>(&inbound_bytes, Addrd(say_exit.into(), addr));
    });

    timeout.wait();
    work.join().unwrap();
    server.join().unwrap();
  }

  #[test]
  fn hello() {
    let (clock, timeout, sock, addr) = setup(Duration::from_secs(1));
    let (inbound_bytes, outbound_bytes) = (sock.rx.clone(), sock.tx.clone());
    let timeout_state = timeout.state.clone();

    let mut say_hello_con = Req::<Test>::get("0.0.0.0", 1234, "hello");
    say_hello_con.set_msg_token(Token(Default::default()));
    say_hello_con.set_msg_id(Id(1));
    let mut say_hello_non = say_hello_con.clone();
    say_hello_non.non();
    say_hello_non.set_msg_token(Token(Default::default()));
    say_hello_non.set_msg_id(Id(2));
    let mut say_exit = Req::<Test>::get("0.0.0.0", 1234, "exit");
    say_exit.set_msg_token(Token(Default::default()));
    say_exit.set_msg_id(Id(2));

    let server = thread::spawn(move || {
      let mut server = TestServer::new(sock, clock);

      server.middleware(&ware::hello);
      server.middleware(&ware::exit);
      server.start().unwrap();

      Timeout::cancel(timeout_state);
    });

    let work = thread::spawn(move || {
      SockMock::send_msg::<Test>(&inbound_bytes, Addrd(say_hello_non.into(), addr));
      let rep: Resp<Test> = SockMock::await_msg::<Test>(addr, &outbound_bytes).into();
      assert_eq!(rep.code(), code::CONTENT);
      assert_eq!(rep.msg_type(), kwap_msg::Type::Non);

      SockMock::send_msg::<Test>(&inbound_bytes, Addrd(say_hello_con.into(), addr));
      let rep: Resp<Test> = SockMock::await_msg::<Test>(addr, &outbound_bytes).into();
      assert_eq!(rep.code(), code::CONTENT);
      assert_eq!(rep.msg_type(), kwap_msg::Type::Ack);

      SockMock::send_msg::<Test>(&inbound_bytes, Addrd(say_exit.into(), addr));
    });

    timeout.wait();
    server.join().unwrap();
    work.join().unwrap();
  }
}
