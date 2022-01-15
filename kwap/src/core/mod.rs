use core::cell::RefCell;
use core::str::FromStr;

use kwap_common::Array;
use kwap_msg::TryIntoBytes;
use no_std_net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use tinyvec::ArrayVec;

/// Events used by core
pub mod event;
use event::listeners::{resp_from_msg, try_parse_message};
use event::{Event, Eventer, MatchEvent};

use self::event::listeners::log;
use crate::config::{self, Config};
use crate::req::Req;
use crate::resp::Resp;
use crate::result_ext::ResultExt;
use crate::socket::Socket;

/// A queued ACK message
#[derive(Debug, Clone, Copy)]
pub struct ToAck {
  /// Address to send ACK to
  pub addr: SocketAddr,
  /// Message to ACK
  pub id: kwap_msg::Id,
}

impl ToAck {
  fn msg<Cfg: Config>(&self) -> config::Message<Cfg> {
    config::Message::<Cfg> { id: self.id,
                             token: kwap_msg::Token(Default::default()),
                             ver: Default::default(),
                             ty: kwap_msg::Type::Ack,
                             code: kwap_msg::Code::new(0, 0),
                             payload: kwap_msg::Payload(Default::default()),
                             opts: Default::default() }
  }
}

/// A CoAP request/response runtime that drives client- and server-side behavior.
///
/// Defined as a state machine with state transitions ([`Event`]s).
///
/// The behavior at runtime is fully customizable, with the default behavior provided via [`Core::new()`](#method.new).
#[allow(missing_debug_implementations)]
pub struct Core<Sock: Socket, Cfg: Config> {
  sock: Sock,
  // Option for these collections provides a Default implementation,
  // which is required by ArrayVec.
  //
  // This also allows us efficiently take owned responses from the collection without reindexing the other elements.
  ears: ArrayVec<[Option<(MatchEvent, fn(&mut Self, &mut Event<Cfg>))>; 16]>,
  emptys: ArrayVec<[Option<(config::Message<Cfg>, SocketAddr)>; 8]>,
  resps: ArrayVec<[Option<(Resp<Cfg>, SocketAddr)>; 16]>,
  ack_queue: ArrayVec<[Option<ToAck>; 16]>,
}

/// An error encounterable while sending a message
#[derive(Debug, Clone)]
pub enum SendError<Cfg: Config, Sock: Socket> {
  /// Some socket operation (e.g. connecting to host) failed
  SockError(Sock::Error),
  /// Serializing a message to bytes failed
  ToBytes(<config::Message<Cfg> as TryIntoBytes>::Error),
  /// Uri-Host in request was not a utf8 string
  HostInvalidUtf8(core::str::Utf8Error),
  /// Uri-Host in request was not a valid IPv4 address (TODO)
  HostInvalidIpAddress,
}

impl<Sock: Socket, Cfg: Config> Core<Sock, Cfg> {
  /// Creates a new Core with the default runtime behavior
  pub fn new(sock: Sock) -> Self {
    let mut me = Self::behaviorless(sock);
    me.bootstrap();
    me
  }

  /// Create a new runtime without any actual behavior
  ///
  /// ```
  /// use std::net::UdpSocket;
  ///
  /// use kwap::config::Alloc;
  /// use kwap::core::Core;
  ///
  /// let sock = UdpSocket::bind("0.0.0.0:12345").unwrap();
  /// Core::<UdpSocket, Alloc>::behaviorless(sock);
  /// ```
  pub fn behaviorless(sock: Sock) -> Self {
    Self { sock,
           ears: Default::default(),
           resps: Default::default(),
           emptys: Default::default(),
           ack_queue: Default::default() }
  }

  /// Add the default behavior to a behaviorless Core
  ///
  /// # Example
  /// See `./examples/client.rs`
  ///
  /// # Event handlers registered
  ///
  /// | Event type | Handler | Should then fire | Remarks |
  /// | -- | -- | -- | -- |
  /// | [`Event::RecvDgram`] | [`try_parse_message`] | [`Event::MsgParseError`] or [`Event::RecvMsg`] | None |
  /// | [`Event::MsgParseError`] | [`log`] | None | only when crate feature `no_std` is not enabled |
  /// | [`Event::RecvMsg`] | [`resp_from_msg`] | [`Event::RecvResp`] or nothing | None |
  /// | [`Event::RecvResp`] | [`Core::store_resp`](#method.store_resp) | nothing (yet) | Manages internal state used to match request ids (see [`Core.poll_resp()`](#method.poll_resp)) |
  ///
  /// ```
  /// use std::net::UdpSocket;
  ///
  /// use kwap::config::Alloc;
  /// use kwap::core::Core;
  ///
  /// let sock = UdpSocket::bind(("0.0.0.0", 8003)).unwrap();
  ///
  /// // Note: this is the same as Core::new().
  /// let mut core = Core::<_, Alloc>::behaviorless(sock);
  /// core.bootstrap()
  /// ```
  pub fn bootstrap(&mut self) {
    self.listen(MatchEvent::RecvDgram, try_parse_message);
    #[cfg(any(test, not(feature = "no_std")))]
    self.listen(MatchEvent::MsgParseError, log);
    self.listen(MatchEvent::RecvMsg, resp_from_msg);
    self.listen(MatchEvent::All, Self::queue_ack);
    self.listen(MatchEvent::RecvMsg, Self::store_empty);
    self.listen(MatchEvent::RecvResp, Self::store_resp);
  }

  /// ACK all the CON responses we've received
  pub fn process_ack_queue(&mut self) -> Result<(), SendError<Cfg, Sock>> {
    let mut iter = self.ack_queue.iter_mut().filter_map(Option::take);

    while let Some(ack) = iter.next() {
      let bytes = ack.msg::<Cfg>()
                     .try_into_bytes::<ArrayVec<[u8; 1152]>>()
                     .map_err(SendError::ToBytes)?;

      Self::send(&mut self.sock, ack.addr, bytes)?;
    }

    Ok(())
  }

  /// Listens for incoming CONfirmable messages and places them on a queue to reply to with ACKs.
  ///
  /// These ACKs are processed whenever the socket is polled (e.g. [`poll_resp`](#method.poll_resp))
  ///
  /// # Panics
  /// panics when msg storage limit reached (e.g. we receive >16 CON requests and have not acked any)
  pub fn queue_ack(&mut self, ev: &mut Event<Cfg>) {
    match ev {
      | Event::RecvResp(Some((ref resp, ref addr))) => {
        if resp.msg_type() == kwap_msg::Type::Con {
          self.ack_queue.push(Some(ToAck { id: resp.msg_id(),
                                           addr: *addr }));
        }
      },
      | _ => {},
    }
  }

  /// Listens for RecvMsg events that are not requests or responses, and stores them on the runtime struct
  ///
  /// # Panics
  /// panics when msg storage limit reached (e.g. 64 pings were sent and we haven't polled for a response of a single one)
  pub fn store_empty(&mut self, ev: &mut Event<Cfg>) {
    let msg = ev.get_mut_msg().unwrap();

    if msg.is_some() && msg.as_ref().unwrap().0.code == kwap_msg::Code::new(0, 0) {
      let msg = msg.take().unwrap();
      // this is not as smart as store_resp because empty messages are much less common
      self.emptys.push(Some(msg));
    }
  }

  /// Listens for RecvResp events and stores them on the runtime struct
  ///
  /// # Panics
  /// panics when response tracking limit reached (e.g. 64 requests were sent and we haven't polled for a response of a single one)
  pub fn store_resp(&mut self, ev: &mut Event<Cfg>) {
    let resp = ev.get_mut_resp().unwrap().take().unwrap();
    if let Some(resp) = self.resps.try_push(Some(resp)) {
      // arrayvec is full, remove nones
      self.resps = self.resps.iter_mut().filter_map(|o| o.take()).map(Some).collect();

      // panic if we're still full
      self.resps.push(resp);
    }
  }

  /// Listen for an event
  ///
  /// # Example
  /// See [`Core.fire()`](#method.fire)
  pub fn listen(&mut self, mat: MatchEvent, listener: fn(&mut Self, &mut Event<Cfg>)) {
    self.ears.push(Some((mat, listener)));
  }

  /// Fire an event
  ///
  /// ```
  /// use std::net::UdpSocket;
  ///
  /// use kwap::config::Alloc;
  /// use kwap::core::event::{Event, MatchEvent};
  /// use kwap::core::Core;
  /// use kwap_msg::MessageParseError::UnexpectedEndOfStream;
  ///
  /// static mut LOG_ERRS_CALLS: u8 = 0;
  ///
  /// fn log_errs(_: &mut Core<UdpSocket, Alloc>, ev: &mut Event<Alloc>) {
  ///   let err = ev.get_msg_parse_error().unwrap();
  ///   eprintln!("error! {:?}", err);
  ///   unsafe {
  ///     LOG_ERRS_CALLS += 1;
  ///   }
  /// }
  ///
  /// let sock = UdpSocket::bind("0.0.0.0:12345").unwrap();
  /// let mut client = Core::behaviorless(sock);
  ///
  /// client.listen(MatchEvent::MsgParseError, log_errs);
  /// client.fire(Event::<Alloc>::MsgParseError(UnexpectedEndOfStream));
  ///
  /// unsafe { assert_eq!(LOG_ERRS_CALLS, 1) }
  /// ```
  pub fn fire(&mut self, event: Event<Cfg>) {
    let mut sound = event;
    let ears: ArrayVec<[_; 16]> = self.ears.iter().copied().collect();

    ears.into_iter().filter_map(|o| o).for_each(|(mat, work)| {
                                        if mat.matches(&sound) {
                                          work(self, &mut sound);
                                        }
                                      });
  }

  /// Poll for a response to a sent request
  ///
  /// # Example
  /// See `./examples/client.rs`
  pub fn poll_resp(&mut self, token: kwap_msg::Token, sock: SocketAddr) -> nb::Result<Resp<Cfg>, SendError<Cfg, Sock>> {
    self.poll(kwap_msg::Id(0), sock, token, Self::try_get_resp)
  }

  /// Poll for an empty message in response to a sent empty message (CoAP ping)
  ///
  /// ```text
  /// Client    Server
  ///  |        |
  ///  |        |
  ///  +------->|     Header: EMPTY (T=CON, Code=0.00, MID=0x0001)
  ///  | EMPTY  |      Token: 0x20
  ///  |        |
  ///  |        |
  ///  |<-------+     Header: RESET (T=RST, Code=0.00, MID=0x0001)
  ///  | 0.00   |      Token: 0x20
  ///  |        |
  /// ```
  pub fn poll_ping(&mut self,
                   req_id: kwap_msg::Id,
                   addr: SocketAddr)
                   -> nb::Result<config::Message<Cfg>, SendError<Cfg, Sock>> {
    self.poll(req_id, addr, kwap_msg::Token(Default::default()), Self::try_get_empty)
  }

  fn poll<R>(&mut self,
             req_id: kwap_msg::Id,
             addr: SocketAddr,
             token: kwap_msg::Token,
             f: fn(&mut Self, kwap_msg::Id, kwap_msg::Token, SocketAddr) -> nb::Result<R, Sock::Error>)
             -> nb::Result<R, SendError<Cfg, Sock>> {
    // check if there's a dgram in the socket,
    // and move it through the event pipeline.
    //
    // this will store the response (if there is one) before we continue.
    self.sock
        .poll()
        .map(|polled| {
          if let Some(dgram) = polled {
            self.fire(Event::RecvDgram(Some(dgram)));
          }
          ()
        })
        .map_err(SendError::SockError)
        .try_perform(|_| self.process_ack_queue())
        .map_err(nb::Error::Other)
        .bind(|_| f(self, req_id, token, addr).map_err(|e| e.map(SendError::SockError)))
  }

  fn try_get_resp(&mut self,
                  _: kwap_msg::Id,
                  token: kwap_msg::Token,
                  sock: SocketAddr)
                  -> nb::Result<Resp<Cfg>, Sock::Error> {
    let resp_matches = |o: &Option<(Resp<Cfg>, SocketAddr)>| {
      let (resp, sock_stored) = o.as_ref().unwrap();
      resp.msg.token == token && *sock_stored == sock
    };

    self.resps
        .iter_mut()
        .find_map(|rep| match rep {
          | mut o @ Some(_) if resp_matches(&o) => Option::take(&mut o).map(|(resp, _)| resp),
          | _ => None,
        })
        .ok_or(nb::Error::WouldBlock)
  }

  fn try_get_empty(&mut self,
                   req_id: kwap_msg::Id,
                   _: kwap_msg::Token,
                   addr: SocketAddr)
                   -> nb::Result<config::Message<Cfg>, Sock::Error> {
    let msg_matches = |o: &Option<(config::Message<Cfg>, SocketAddr)>| {
      let (msg, stored_addr) = o.as_ref().unwrap();
      msg.id == req_id && *stored_addr == addr
    };

    self.emptys
        .iter_mut()
        .find_map(|rep| match rep {
          | mut o @ Some(_) if msg_matches(&o) => Option::take(&mut o).map(|(msg, _)| msg),
          | _ => None,
        })
        .ok_or(nb::Error::WouldBlock)
  }

  /// Send a request!
  ///
  /// ```
  /// use std::net::UdpSocket;
  ///
  /// use kwap::config::Alloc;
  /// use kwap::core::Core;
  /// use kwap::req::Req;
  ///
  /// let sock = UdpSocket::bind(("0.0.0.0", 8002)).unwrap();
  /// let mut core = Core::<_, Alloc>::new(sock);
  /// core.send_req(Req::<Alloc>::get("1.1.1.1", 5683, "/hello"));
  /// ```
  pub fn send_req(&mut self, req: Req<Cfg>) -> Result<(kwap_msg::Token, SocketAddr), SendError<Cfg, Sock>> {
    let token = req.msg_token();
    let port = req.get_option(7).expect("Uri-Port must be present");
    let port_bytes = port.value.0.iter().take(2).copied().collect::<ArrayVec<[u8; 2]>>();
    let port = u16::from_be_bytes(port_bytes.into_inner());

    let host: ArrayVec<[u8; 128]> = req.get_option(3)
                                       .expect("Uri-Host must be present")
                                       .value
                                       .0
                                       .iter()
                                       .copied()
                                       .collect();
    core::str::from_utf8(&host).map_err(SendError::HostInvalidUtf8)
                               .bind(|host| Ipv4Addr::from_str(host).map_err(|_| SendError::HostInvalidIpAddress))
                               .tupled(|_| req.try_into_bytes::<ArrayVec<[u8; 1152]>>().map_err(SendError::ToBytes))
                               .map(|(host, bytes)| (SocketAddr::V4(SocketAddrV4::new(host, port)), bytes))
                               .bind(|(addr, bytes)| Self::send(&mut self.sock, addr, bytes))
                               .map(|addr| (token, addr))
  }

  /// Send a ping message to some remote coap server
  /// to check liveness.
  ///
  /// Returns a message id that can be used to poll for the response
  /// via [`poll_ping`](#method.poll_ping)
  ///
  /// ```
  /// use std::net::UdpSocket;
  ///
  /// use kwap::config::Alloc;
  /// use kwap::core::Core;
  /// use kwap::req::Req;
  ///
  /// let sock = UdpSocket::bind(("0.0.0.0", 8004)).unwrap();
  /// let mut core = Core::<_, Alloc>::new(sock);
  /// let id = core.ping("1.1.1.1", 5683);
  /// // core.poll_ping(id);
  /// ```
  pub fn ping(&mut self, host: impl AsRef<str>, port: u16) -> Result<(kwap_msg::Id, SocketAddr), SendError<Cfg, Sock>> {
    let mut msg: config::Message<Cfg> = Req::<Cfg>::get(host.as_ref(), port, "").into();
    msg.token = kwap_msg::Token(Default::default());
    msg.opts = Default::default();
    msg.code = kwap_msg::Code::new(0, 0);

    let id = msg.id;
    msg.try_into_bytes::<ArrayVec<[u8; 13]>>()
       .map_err(SendError::ToBytes)
       .tupled(|_| Ipv4Addr::from_str(host.as_ref()).map_err(|_| SendError::HostInvalidIpAddress))
       .bind(|(bytes, host)| Self::send(&mut self.sock, SocketAddr::V4(SocketAddrV4::new(host, port)), bytes))
       .map(|addr| (id, addr))
  }

  /// Send a raw message down the wire to some remote host.
  ///
  /// You probably want [`send_req`](#method.send_req) or [`ping`](#method.ping) instead.
  fn send(sock: &mut Sock, addr: SocketAddr, bytes: impl Array<Item = u8>) -> Result<SocketAddr, SendError<Cfg, Sock>> {
    // TODO: uncouple from ipv4
    sock.connect(addr)
        .map_err(SendError::SockError)
        .try_perform(|_| nb::block!(sock.send(&bytes)).map_err(SendError::SockError))
        .map(|_| addr)
  }
}

impl<Sock: Socket, Cfg: Config> Eventer<Cfg> for Core<Sock, Cfg> {
  fn fire(&mut self, ev: Event<Cfg>) {
    Self::fire(self, ev)
  }

  fn listen(&mut self, mat: MatchEvent, f: fn(&mut Self, &mut Event<Cfg>)) {
    self.listen(mat, f)
  }
}

#[cfg(test)]
mod tests {
  use kwap_msg::TryIntoBytes;
  use tinyvec::ArrayVec;

  use super::*;
  use crate::config;
  use crate::config::Alloc;
  use crate::req::Req;
  use crate::test::TubeSock;

  #[test]
  fn eventer() {
    let req = Req::<Alloc>::get("0.0.0.0", 1234, "");
    let bytes = config::Message::<Alloc>::from(req).try_into_bytes::<ArrayVec<[u8; 1152]>>()
                                                   .unwrap();
    let mut client = Core::<TubeSock, Alloc>::behaviorless(TubeSock::new());

    fn on_err(_: &mut Core<TubeSock, Alloc>, e: &mut Event<Alloc>) {
      panic!("{:?}", e)
    }

    static mut CALLS: usize = 0;
    fn on_dgram(_: &mut Core<TubeSock, Alloc>, _: &mut Event<Alloc>) {
      unsafe {
        CALLS += 1;
      }
    }

    client.listen(MatchEvent::MsgParseError, on_err);
    client.listen(MatchEvent::RecvDgram, on_dgram);

    let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 1234);
    client.fire(Event::RecvDgram(Some((bytes, addr.into()))));

    unsafe {
      assert_eq!(CALLS, 1);
    }
  }

  #[test]
  fn ping() {
    type Msg = config::Message<Alloc>;

    let mut client = Core::<TubeSock, Alloc>::new(TubeSock::new());
    let (id, addr) = client.ping("0.0.0.0", 5632).unwrap();

    let resp = Msg { id,
                     token: kwap_msg::Token(Default::default()),
                     code: kwap_msg::Code::new(0, 0),
                     ver: Default::default(),
                     ty: kwap_msg::Type::Reset,
                     payload: kwap_msg::Payload(Default::default()),
                     opts: Default::default() };

    let bytes = resp.try_into_bytes::<ArrayVec<[u8; 1152]>>().unwrap();

    client.fire(Event::RecvDgram(Some((bytes, addr))));
    let rep = client.poll_ping(id, addr.into()).unwrap();
    assert_eq!(bytes, rep.try_into_bytes::<ArrayVec<[u8; 1152]>>().unwrap());
  }

  #[test]
  fn client_flow() {
    type Msg = config::Message<Alloc>;

    let req = Req::<Alloc>::get("0.0.0.0", 1234, "");
    let token = req.msg.token;
    let resp = Resp::<Alloc>::for_request(req);
    let bytes = Msg::from(resp).try_into_bytes::<Vec<u8>>().unwrap();

    let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 1234);
    let mut client = Core::<TubeSock, Alloc>::new(TubeSock::init(addr.clone().into(), bytes.clone()));

    let rep = client.poll_resp(token, addr.into()).unwrap();
    assert_eq!(bytes, Msg::from(rep).try_into_bytes::<Vec<u8>>().unwrap());
  }
}
