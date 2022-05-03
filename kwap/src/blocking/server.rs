use kwap_common::Array;

use crate::config::{Config, Std, self};
use crate::core::Core;
use crate::req::Req;
use crate::socket::Addressed;

/// Type yielded by server middleware (e.g. resource handlers)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Continuation {
  /// This message should continue to be processed by the next middleware.
  Continue,
  /// This message has been fully handled and should not be processed
  /// by any other middleware.
  Stop,
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
pub struct Server<'a, Cfg: Config, Middleware: Array<Item = &'a dyn Fn(Addressed<Req<Cfg>>) -> (Continuation, Action<Cfg>)>> {
  core: Core<Cfg>,
  middleware: Middleware,
}

impl<'a> Server<'a, Std, Vec<&'a dyn Fn(Addressed<Req<Std>>) -> (Continuation, Action<Std>)>> {
  /// Create a new Server
  ///
  /// ```no_run
  /// use kwap::config::{Std, Message};
  /// use kwap::socket::Addressed;
  /// use kwap::option::ContentFormat;
  /// use kwap::req::Req;
  /// use kwap::resp::{Resp, code};
  ///
  /// use kwap::blocking::server::{Server, Continuation, Action};
  ///
  /// fn hello(req: Addressed<Req<Std>>) -> (Continuation, Action<Std>) {
  ///    match req.route() {
  ///      "hello" => {
  ///        let mut resp = Resp::for_request(req.data().clone());
  ///        resp.set_code(code::CONTENT);
  ///
  ///        resp.set_option(
  ///          12, // Content-Format
  ///          Some(ContentFormat::Json.into())
  ///        );
  ///
  ///        let payload = r#"{ "hello": "world" }"#;
  ///        resp.set_payload(payload.bytes());
  ///
  ///        let msg: Addressed<Message<Std>> = req.as_ref().map(|_| resp.into());
  ///
  ///        (Stop, Send(msg))
  ///      },
  ///      _ => (Continue, Nop)
  ///    }
  /// }
  ///
  /// fn not_found(req: Addressed<Req<Std>>) -> (Continuation, Action<Std>) {
  ///   let resp = Resp::for_request(req.data().clone());
  ///   resp.set_code(code::NOT_FOUND);
  ///   let msg: Addressed<Message<Std>> = req.as_ref().map(|_| resp.into());
  ///   (Stop, Send(msg))
  /// }
  ///
  /// let server = Server::<Std>::try_new([127, 0, 0, 1], 3030);
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

#[cfg(test)]
mod tests { }
