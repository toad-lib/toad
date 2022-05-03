use kwap_common::Array;

use crate::config::{Config, Std, self};
use crate::core::Core;
use crate::req::Req;
use crate::socket::Addressed;

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

/// Foo
#[allow(missing_debug_implementations)]
pub struct Server<'a, Cfg: Config, Middleware: Array<Item = &'a dyn Fn(Addressed<Req<Cfg>>) -> (Continue, Action<Cfg>)>> {
  core: Core<Cfg>,
  middleware: Middleware,
}

impl<'a> Server<'a, Std, Vec<&'a dyn Fn(Addressed<Req<Std>>) -> (Continue, Action<Std>)>> {
  /// Create a new Server
  ///
  /// ```no_run
  /// use kwap::ContentFormat;
  /// use kwap::config::{Std, Message};
  /// use kwap::socket::Addressed;
  /// use kwap::req::Req;
  /// use kwap::resp::{Resp, code};
  ///
  /// use kwap::blocking::server::{Server, Continue, Action};
  ///
  /// fn hello(req: Addressed<Req<Std>>) -> (Continue, Action<Std>) {
  ///    match req.data().path() {
  ///      Ok(Some("hello")) => {
  ///        let mut resp = Resp::for_request(req.data().clone());
  ///        resp.set_code(code::CONTENT);
  ///
  ///        resp.set_option(
  ///          12, // Content-Format
  ///          ContentFormat::Json.bytes()
  ///        );
  ///
  ///        let payload = r#"{ "hello": "world" }"#;
  ///        resp.set_payload(payload.bytes());
  ///
  ///        let msg: Addressed<Message<Std>> = req.as_ref().map(|_| resp.into());
  ///
  ///        (Continue::No, Action::Send(msg))
  ///      },
  ///      _ => (Continue::Yes, Action::Nop)
  ///    }
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
  pub fn try_new([a, b, c, d]: [u8; 4], port: u16) -> Result<Self, std::io::Error> {
    let clock = crate::std::Clock::new();

    let ip = std::net::Ipv4Addr::new(a, b, c, d);
    let sock = std::net::UdpSocket::bind((ip, port))?;
    let core = Core::<Std>::new(clock, sock);

    Ok(Self {core, middleware: vec![]})
  }
}

impl<'a, Cfg: Config, Middleware: Array<Item = &'a dyn Fn(Addressed<Req<Cfg>>) -> (Continue, Action<Cfg>)>> Server<'a, Cfg, Middleware> {
  /// TODO
  pub fn middleware(&mut self, f: <Middleware as Array>::Item) -> () {
    self.middleware.push(f);
  }
}

#[cfg(test)]
mod tests { }
