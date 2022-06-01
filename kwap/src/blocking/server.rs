use kwap_common::Array;
use kwap_msg::Type;

use crate::config::Config;
use crate::core::{Core, Error};
use crate::net::{Addrd, Socket};
#[cfg(feature = "std")]
use crate::platform::Std;
use crate::platform::{self, Platform};
use crate::req::{Method, Req};
use crate::resp::Resp;

/// Data structure used by server for bookkeeping of
/// "the result of the last middleware run and what to do next"
#[derive(Debug)]
enum Status<Cfg: Platform> {
  Err(Error<Cfg>),
  Continue,
  Done,
  Exit,
}

impl<Cfg: Platform> Status<Cfg> {
  fn bind_result(self, result: Result<(), Error<Cfg>>) -> Self {
    match &self {
      | Self::Err(_) => self,
      | Self::Done | Self::Exit | Self::Continue => result.map(|_| self)
                                                          .map_err(|e| Self::Err(e))
                                                          .unwrap_or_else(|e| e),
    }
  }
}

/// Type alias for a server middleware function. See [`Server.middleware`]
pub type Middleware<Cfg> = dyn Fn(&Addrd<Req<Cfg>>) -> Actions<Cfg>;

/// Action to perform as a result of middleware
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum Action<P: Platform> {
  /// Send a response
  SendResp(Addrd<Resp<P>>),
  /// Send a request
  SendReq(Addrd<Req<P>>),
  /// Send a message
  Send(Addrd<platform::Message<P>>),
  /// Stop the server completely
  Exit,
  /// TODO
  Continue,
}

impl<P: Platform> Action<P> {
  /// TODO
  pub fn then(self, action: Action<P>) -> Actions<P> {
    Actions::just(self).then(action)
  }
}

/// TODO
#[derive(Clone, Debug)]
pub struct Actions<P: Platform>(tinyvec::ArrayVec<[Option<Action<P>>; 16]>);

impl<P: Platform> Actions<P> {
  /// TODO
  pub fn just(action: Action<P>) -> Self {
    action.into()
  }

  /// TODO
  pub fn then(mut self, action: Action<P>) -> Self {
    self.0.push(Some(action));
    self
  }
}

impl<P: Platform> Default for Actions<P> {
  fn default() -> Self {
    Self(Default::default())
  }
}

impl<P: Platform> Into<Actions<P>> for Action<P> {
  fn into(self) -> Actions<P> {
    let mut actions = Actions::default();
    actions.0.push(Some(self));
    actions
  }
}

/// A barebones CoAP server.
///
/// See the documentation for [`Server.try_new`] for example usage.
// TODO(#85): allow opt-out of always piggybacked ack responses
#[allow(missing_debug_implementations)]
pub struct Server<'a, P: Platform, Middlewares: 'static + Array<Item = &'a Middleware<P>>> {
  core: Core<P>,
  fns: Middlewares,
}

#[cfg(feature = "std")]
impl<'a> Server<'a, Std, Vec<&'a Middleware<Std>>> {
  /// Create a new Server
  ///
  /// ```no_run
  /// use kwap::blocking::server::{Action, Actions, Server};
  /// use kwap::net::Addrd;
  /// use kwap::platform::{Message, Std};
  /// use kwap::req::Req;
  /// use kwap::resp::{code, Resp};
  /// use kwap::ContentFormat;
  ///
  /// fn hello(req: &Addrd<Req<Std>>) -> Actions<Std> {
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
  ///       Action::Send(msg).into()
  ///     },
  ///     | _ => Action::Continue.into(),
  ///   }
  /// }
  ///
  /// fn not_found(req: &Addrd<Req<Std>>) -> Actions<Std> {
  ///   let mut resp = Resp::for_request(req.data()).unwrap();
  ///   resp.set_code(code::NOT_FOUND);
  ///   let msg: Addrd<Message<Std>> = req.as_ref().map(|_| resp.into());
  ///   Action::Send(msg).into()
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

    std::net::UdpSocket::bind((ip, port)).map(|sock| {
                                           Self::new_config(config, sock, crate::std::Clock::new())
                                         })
  }
}

impl<'a, P: Platform, Middlewares: 'static + Array<Item = &'a Middleware<P>>>
  Server<'a, P, Middlewares>
{
  /// Construct a new Server for the current platform.
  ///
  /// If the standard library is available, see [`Server.try_new`].
  pub fn new(sock: P::Socket, clock: P::Clock) -> Self {
    Self::new_config(Config::default(), sock, clock)
  }

  /// Construct a new Server for the current platform that listens on the
  /// "All CoAP devices" multicast address.
  pub fn new_multicast(clock: P::Clock, port: u16) -> Result<Self, <P::Socket as Socket>::Error> {
    P::Socket::bind(crate::multicast::all_coap_devices(port)).map(|sock| Self::new(sock, clock))
  }

  /// Create a new server with a specific runtime config
  pub fn new_config(config: Config, sock: P::Socket, clock: P::Clock) -> Self {
    let core = Core::<P>::new_config(config, clock, sock);

    let mut self_ = Self { core,
                           fns: Default::default() };
    self_.middleware(&Self::respond_ping);

    self_
  }

  /// Middleware function that responds to CoAP pings (EMPTY Confirmable messages)
  ///
  /// This is included when Server::new is invoked.
  pub fn respond_ping(req: &Addrd<Req<P>>) -> Actions<P> {
    match (req.data().method(), req.data().msg_type()) {
      | (Method::EMPTY, Type::Con) => {
        let resp = platform::Message::<P> { ver: Default::default(),
                                            ty: Type::Reset,
                                            id: req.data().msg_id(),
                                            token: kwap_msg::Token(Default::default()),
                                            code: kwap_msg::Code::new(0, 0),
                                            opts: Default::default(),
                                            payload: kwap_msg::Payload(Default::default()) };

        Action::Send(req.as_ref().map(|_| resp)).into()
      },
      | _ => Action::Continue.into(),
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
  /// fn hello(Addrd<Req<P>>) -> Actions<P> {
  ///   /*
  ///     path == "hello"
  ///     ? Action::Send(2.05 CONTENT).into()
  ///     : Action::Continue
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
  pub fn middleware(&mut self, f: &'a Middleware<P>) -> () {
    self.fns.push(f);
  }

  fn perform_one(core: &mut Core<P>, action: Action<P>) -> Result<(), Error<P>> {
    match action {
      | Action::Continue => Ok(()),
      | Action::Send(msg) => core.send_msg(msg),
      | Action::SendReq(req) => core.send_addrd_req(req).map(|_| ()),
      | Action::SendResp(resp) => core.send_msg(resp.map(Into::into)),
      | Action::Exit => unreachable!(),
    }
  }

  fn perform_many(core: &mut Core<P>, actions: Actions<P>) -> Status<P> {
    actions.0
           .into_iter()
           .filter_map(|o| o)
           .fold(Status::Continue, |status, action| match (status, action) {
             | (Status::Exit, _) | (_, Action::Exit) => Status::Exit,
             | (Status::Done, Action::Continue) => Status::Continue,
             | (Status::Done, _) => Status::Done,
             | (status @ Status::Err(_), _) => status,
             | (_, action) => Status::Done.bind_result(Self::perform_one(core, action)),
           })
  }

  /// Start the server
  ///
  /// A function may be provided (`on_tick`) that will be called every time
  /// the server checks for an incoming request and finds none.
  ///
  /// This function can be used to send out-of-band messages.
  ///
  /// For example: in the case of a multicast listener, we need to both broadcast
  /// on the multicast address "_Hey! I'm a CoAP server listening on "All CoAP nodes"_"
  /// **and** act as a normal server that receives direct requests.
  pub fn start_tick(&mut self,
                    on_tick: Option<&'a dyn Fn() -> Actions<P>>)
                    -> Result<(), Error<P>> {
    loop {
      let req = loop {
        match self.core.poll_req() {
          | Ok(req) => break Ok(req),
          | Err(nb::Error::Other(e)) => break Err(e),
          | Err(nb::Error::WouldBlock) => {
            // TODO: do something with errors
            if let Some(on_tick) = on_tick {
              let status = Self::perform_many(&mut self.core, on_tick());
              match status {
                | Status::Exit => return Ok(()),
                | Status::Continue => continue,
                | _ => todo!(),
              };
            }
          },
        }
      }?;

      let status = self.fns
                       .iter()
                       .fold(Status::Continue, |status, f| match status {
                         | Status::Exit | Status::Err(_) | Status::Done => status,
                         | Status::Continue => Self::perform_many(&mut self.core, f(&req)),
                       });

      // TODO: do something with errors
      match status {
        | Status::Exit => return Ok(()),
        | _ => continue,
      }
    }
  }

  /// Start the server
  pub fn start(&mut self) -> Result<(), Error<P>> {
    self.start_tick(None)
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
    pub fn panics(_: &Addrd<Req<Test>>) -> Actions<Test> {
      panic!()
    }

    pub fn not_found(req: &Addrd<Req<Test>>) -> Actions<Test> {
      let reply = req.as_ref().map(|req| {
                                let mut resp = Resp::<Test>::for_request(req).unwrap();
                                resp.set_code(code::NOT_FOUND);
                                resp.into()
                              });

      Action::Send(reply).into()
    }

    pub fn hello(req: &Addrd<Req<Test>>) -> Actions<Test> {
      if req.0.method() == Method::GET && req.0.path().unwrap() == Some("hello") {
        let reply = req.as_ref().map(|req| {
                                  let mut resp = Resp::<Test>::for_request(req).unwrap();
                                  resp.set_payload("hello!".bytes());
                                  resp.set_code(code::CONTENT);
                                  resp.into()
                                });

        Action::Send(reply).into()
      } else {
        Action::Continue.into()
      }
    }

    pub fn exit(req: &Addrd<Req<Test>>) -> Actions<Test> {
      if req.0.path().unwrap() == Some("exit") {
        Action::Exit.into()
      } else {
        Action::Continue.into()
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

    let mut say_exit = Req::<Test>::get("0.0.0.0:1234".parse().unwrap(), "exit");
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

    let mut say_hello = Req::<Test>::get("0.0.0.0:1234".parse().unwrap(), "hello");
    say_hello.set_msg_token(Token(Default::default()));
    say_hello.set_msg_id(Id(1));

    let mut say_exit = Req::<Test>::get("0.0.0.0:1234".parse().unwrap(), "exit");
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

    let mut ping = Req::<Test>::new(Method::EMPTY, "0.0.0.0:1234".parse().unwrap(), "");
    ping.set_msg_token(Token(Default::default()));
    ping.set_msg_id(Id(1));
    let mut say_exit = Req::<Test>::get("0.0.0.0:1234".parse().unwrap(), "exit");
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

    let mut say_hello_con = Req::<Test>::get("0.0.0.0:1234".parse().unwrap(), "hello");
    say_hello_con.set_msg_token(Token(Default::default()));
    say_hello_con.set_msg_id(Id(1));
    let mut say_hello_non = say_hello_con.clone();
    say_hello_non.non();
    say_hello_non.set_msg_token(Token(Default::default()));
    say_hello_non.set_msg_id(Id(2));
    let mut say_exit = Req::<Test>::get("0.0.0.0:1234".parse().unwrap(), "exit");
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
