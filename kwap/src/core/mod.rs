use core::str::FromStr;

use embedded_time::Clock;
use kwap_common::Array;
use kwap_msg::{TryIntoBytes, Type};
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
use crate::retry::RetryTimer;
use crate::socket::Socket;

fn mk_ack<Cfg: Config>(id: kwap_msg::Id, addr: SocketAddr) -> Addressed<config::Message<Cfg>> {
  use kwap_msg::*;
  let msg = config::Message::<Cfg> { id,
                                     token: Token(Default::default()),
                                     ver: Default::default(),
                                     ty: Type::Ack,
                                     code: Code::new(0, 0),
                                     payload: Payload(Default::default()),
                                     opts: Default::default() };

  Addressed(msg, addr)
}

#[derive(Debug, Clone, Copy)]
struct Addressed<T>(T, SocketAddr);

#[derive(Debug, Clone, Copy)]
struct Retryable<Cfg: Config, T>(T, RetryTimer<Cfg::Clock>);

/// A CoAP request/response runtime that drives client- and server-side behavior.
///
/// Defined as a state machine with state transitions ([`Event`]s).
///
/// The behavior at runtime is fully customizable, with the default behavior provided via [`Core::new()`](#method.new).
#[allow(missing_debug_implementations)]
pub struct Core<Cfg: Config> {
  /// Networking socket that the CoAP runtime uses
  sock: Cfg::Socket,
  /// Clock used for timing
  clock: Cfg::Clock,
  // Option for these collections provides a Default implementation,
  // which is required by ArrayVec.
  //
  // This also allows us efficiently take owned responses from the collection without reindexing the other elements.
  /// Event listeners
  ears: ArrayVec<[Option<(MatchEvent, fn(&mut Self, &mut Event<Cfg>))>; 16]>,
  /// Received responses
  resps: ArrayVec<[Option<Addressed<Resp<Cfg>>>; 16]>,
  /// Queue of messages to send whose receipt we do not need to guarantee (NON, ACK)
  non_q: ArrayVec<[Option<Addressed<config::Message<Cfg>>>; 16]>,
  /// Queue of confirmable messages to send at our earliest convenience
  con_q: ArrayVec<[Option<Retryable<Cfg, Addressed<config::Message<Cfg>>>>; 16]>,
}

/// An error encounterable while sending a message
#[derive(Debug)]
pub enum Error<Cfg: Config> {
  /// Some socket operation (e.g. connecting to host) failed
  SockError(<<Cfg as Config>::Socket as Socket>::Error),
  /// Serializing a message to bytes failed
  ToBytes(<config::Message<Cfg> as TryIntoBytes>::Error),
  /// Uri-Host in request was not a utf8 string
  HostInvalidUtf8(core::str::Utf8Error),
  /// Uri-Host in request was not a valid IPv4 address (TODO)
  HostInvalidIpAddress,
  /// A CONfirmable message was sent many times without an ACKnowledgement.
  MessageNeverAcked,
  /// The clock failed to provide timing.
  ///
  /// See [`embedded_time::clock::Error`]
  ClockError,
}

impl<Cfg: Config> Core<Cfg> {
  /// Creates a new Core with the default runtime behavior
  pub fn new(clock: Cfg::Clock, sock: Cfg::Socket) -> Self {
    let mut me = Self::behaviorless(clock, sock);
    me.bootstrap();
    me
  }

  /// Create a new runtime without any actual behavior
  ///
  /// ```
  /// use std::net::UdpSocket;
  ///
  /// use kwap::config::Std;
  /// use kwap::core::Core;
  /// use kwap::std::Clock;
  ///
  /// let sock = UdpSocket::bind("0.0.0.0:12345").unwrap();
  /// Core::<Std>::behaviorless(Clock::new(), sock);
  /// ```
  pub fn behaviorless(clock: Cfg::Clock, sock: Cfg::Socket) -> Self {
    Self { sock,
           clock,
           ears: Default::default(),
           resps: Default::default(),
           non_q: Default::default(),
           con_q: Default::default() }
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
  /// use kwap::config::Std;
  /// use kwap::core::Core;
  /// use kwap::std::Clock;
  ///
  /// let sock = UdpSocket::bind(("0.0.0.0", 8003)).unwrap();
  ///
  /// // Note: this is the same as Core::new().
  /// let mut core = Core::<Std>::behaviorless(Clock::new(), sock);
  /// core.bootstrap()
  /// ```
  pub fn bootstrap(&mut self) {
    self.listen(MatchEvent::RecvDgram, try_parse_message);
    #[cfg(any(test, not(feature = "no_std")))]
    self.listen(MatchEvent::MsgParseError, log);
    self.listen(MatchEvent::RecvMsg, resp_from_msg);
    //          vvvvvvvvvvvvvvv RecvResp and RecvReq
    self.listen(MatchEvent::All, Self::ack);
    self.listen(MatchEvent::RecvMsg, Self::process_acks);
    self.listen(MatchEvent::RecvResp, Self::store_resp);
  }

  /// Process all the queued outbound nonconfirmable messages.
  ///
  /// Notably, unlike `send_cons`, this eagerly takes
  /// the messages out of the queue and discards them after sending,
  /// since we do not need to guarantee receipt of these messages.
  pub fn send_nons(&mut self) -> Result<(), Error<Cfg>> {
    self.non_q
        .iter_mut()
        .filter_map(Option::take)
        .map(|Addressed(msg, addr)| {
          msg.try_into_bytes::<ArrayVec<[u8; 1152]>>()
             .map_err(Error::ToBytes)
             .bind(|bytes| Self::send(&mut self.sock, addr, bytes))
             .map(|_| ())
        })
        .collect()
  }

  /// Process all the queued outbound confirmable messages.
  ///
  /// Notably, unlike `send_nons`, this does not eagerly take
  /// the messages out of the queue and instead clones them and mutates
  /// the RetryTimer in-place.
  ///
  /// The expectation is that when they are Acked, an event handler
  /// will remove them from storage, meaning that a message in con_q
  /// has not been acked yet.
  pub fn send_cons(&mut self) -> Result<(), Error<Cfg>> {
    use crate::retry::YouShould;

    self.con_q
        .iter_mut()
        .filter_map(|o| o.as_mut())
        .map(|Retryable(Addressed(msg, addr), retry)| {
          msg.clone()
             .try_into_bytes::<ArrayVec<[u8; 1152]>>()
             .map_err(Error::ToBytes)
             .tupled(|_| {
               self.clock
                   .try_now()
                   .map_err(|_| Error::ClockError)
                   .map(|now| retry.what_should_i_do(now))
             })
             .bind(|(bytes, should)| match should {
               | Ok(YouShould::Retry) => Self::send(&mut self.sock, *addr, bytes).map(|_| ()),
               | Ok(YouShould::Cry) => Err(Error::MessageNeverAcked),
               | Err(nb::Error::WouldBlock) => Ok(()),
               | _ => unreachable!(),
             })
        })
        .collect()
  }

  /// Listens for incoming CONfirmable messages and places them on a queue to reply to with ACKs.
  ///
  /// These ACKs are processed whenever the socket is polled (e.g. [`poll_resp`](#method.poll_resp))
  ///
  /// # Panics
  /// panics when msg storage limit reached (e.g. we receive >16 CON requests and have not acked any)
  pub fn ack(&mut self, ev: &mut Event<Cfg>) {
    match ev {
      | Event::RecvResp(Some((ref resp, ref addr))) => {
        if resp.msg_type() == kwap_msg::Type::Con {
          self.non_q.push(Some(mk_ack::<Cfg>(resp.msg_id(), *addr)));
        }
      },
      | _ => {},
    }
  }

  /// Listens for incoming ACKs and removes any matching CON messages queued for retry.
  ///
  /// # Panics
  /// panics when msg storage limit reached (e.g. 64 pings were sent and we haven't polled for a response of a single one)
  pub fn process_acks(&mut self, ev: &mut Event<Cfg>) {
    let msg = ev.get_mut_msg().unwrap();

    if msg.is_some() && msg.as_ref().unwrap().0.ty == Type::Ack {
      let (msg, addr) = msg.clone().unwrap();
      if let Some((ix, _)) =
        self.con_q
            .iter()
            .filter_map(|o| o.as_ref())
            .enumerate()
            .find(|(ix, Retryable(Addressed(con, con_addr), _))| *con_addr == addr && con.id == msg.id)
      {
        self.con_q.remove(ix);
      }
    }
  }

  /// Listens for RecvResp events and stores them on the runtime struct
  ///
  /// # Panics
  /// panics when response tracking limit reached (e.g. 64 requests were sent and we haven't polled for a response of a single one)
  pub fn store_resp(&mut self, ev: &mut Event<Cfg>) {
    let resp = ev.get_mut_resp().unwrap().take().unwrap();
    if let Some(resp) = self.resps.try_push(Some(Addressed(resp.0, resp.1))) {
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
  /// use kwap::config::Std;
  /// use kwap::core::event::{Event, MatchEvent};
  /// use kwap::core::Core;
  /// use kwap::std::Clock;
  /// use kwap_msg::MessageParseError::UnexpectedEndOfStream;
  ///
  /// static mut LOG_ERRS_CALLS: u8 = 0;
  ///
  /// fn log_errs(_: &mut Core<Std>, ev: &mut Event<Std>) {
  ///   let err = ev.get_msg_parse_error().unwrap();
  ///   eprintln!("error! {:?}", err);
  ///   unsafe {
  ///     LOG_ERRS_CALLS += 1;
  ///   }
  /// }
  ///
  /// let sock = UdpSocket::bind("0.0.0.0:12345").unwrap();
  /// let mut client = Core::behaviorless(Clock::new(), sock);
  ///
  /// client.listen(MatchEvent::MsgParseError, log_errs);
  /// client.fire(Event::<Std>::MsgParseError(UnexpectedEndOfStream));
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
  pub fn poll_resp(&mut self, token: kwap_msg::Token, sock: SocketAddr) -> nb::Result<Resp<Cfg>, Error<Cfg>> {
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
  pub fn poll_ping(&mut self, req_id: kwap_msg::Id, addr: SocketAddr) -> nb::Result<(), Error<Cfg>> {
    self.poll(req_id, addr, kwap_msg::Token(Default::default()), Self::check_ping)
  }

  fn poll<R>(&mut self,
             req_id: kwap_msg::Id,
             addr: SocketAddr,
             token: kwap_msg::Token,
             f: fn(&mut Self,
                kwap_msg::Id,
                kwap_msg::Token,
                SocketAddr) -> nb::Result<R, <<Cfg as Config>::Socket as Socket>::Error>)
             -> nb::Result<R, Error<Cfg>> {
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
        .map_err(Error::SockError)
        .try_perform(|_| self.send_nons())
        .try_perform(|_| self.send_cons())
        .map_err(nb::Error::Other)
        .bind(|_| f(self, req_id, token, addr).map_err(|e| e.map(Error::SockError)))
  }

  fn try_get_resp(&mut self,
                  _: kwap_msg::Id,
                  token: kwap_msg::Token,
                  sock: SocketAddr)
                  -> nb::Result<Resp<Cfg>, <<Cfg as Config>::Socket as Socket>::Error> {
    let resp_matches = |o: &Option<Addressed<Resp<Cfg>>>| {
      let Addressed(resp, sock_stored) = o.as_ref().unwrap();
      resp.msg.token == token && *sock_stored == sock
    };

    self.resps
        .iter_mut()
        .find_map(|rep| match rep {
          | mut o @ Some(_) if resp_matches(&o) => Option::take(&mut o).map(|Addressed(resp, _)| resp),
          | _ => None,
        })
        .ok_or(nb::Error::WouldBlock)
  }

  fn check_ping(&mut self,
                req_id: kwap_msg::Id,
                _: kwap_msg::Token,
                addr: SocketAddr)
                -> nb::Result<(), <<Cfg as Config>::Socket as Socket>::Error> {
    let still_qd = self.con_q
                       .iter()
                       .filter_map(|o| o.as_ref())
                       .any(|Retryable(Addressed(con, con_addr), _)| con.id == req_id && addr == *con_addr);

    if still_qd {
      Err(nb::Error::WouldBlock)
    } else {
      Ok(())
    }
  }

  /// Send a request!
  ///
  /// ```
  /// use std::net::UdpSocket;
  ///
  /// use kwap::config::Std;
  /// use kwap::core::Core;
  /// use kwap::req::Req;
  ///
  /// let sock = UdpSocket::bind(("0.0.0.0", 8002)).unwrap();
  /// let mut core = Core::<Std>::new(Default::default(), sock);
  /// core.send_req(Req::<Std>::get("1.1.1.1", 5683, "/hello"));
  /// ```
  pub fn send_req(&mut self, req: Req<Cfg>) -> Result<(kwap_msg::Token, SocketAddr), Error<Cfg>> {
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

    let msg = config::Message::<Cfg>::from(req);

    core::str::from_utf8(&host).map_err(Error::HostInvalidUtf8)
                               .bind(|host| Ipv4Addr::from_str(host).map_err(|_| Error::HostInvalidIpAddress))
                               .map(|host| SocketAddr::V4(SocketAddrV4::new(host, port)))
                               .try_perform(|addr| {
                                 if msg.ty == Type::Con {
                                   let t = Addressed(msg.clone(), *addr);
                                   self.retryable(t).map(|bam| self.con_q.push(Some(bam)))
                                 } else {
                                   Ok(())
                                 }
                               })
                               .tupled(|_| msg.try_into_bytes::<ArrayVec<[u8; 1152]>>().map_err(Error::ToBytes))
                               .bind(|(addr, bytes)| Self::send(&mut self.sock, addr, bytes))
                               .map(|addr| (token, addr))
  }

  fn retryable<T>(&self, t: T) -> Result<Retryable<Cfg, T>, Error<Cfg>> {
    self.clock
        .try_now()
        .map(|now| {
          RetryTimer::new(now,
                          crate::retry::Strategy::Exponential(embedded_time::duration::Milliseconds(100)),
                          crate::retry::Attempts(5))
        })
        .map_err(|_| Error::ClockError)
        .map(|timer| Retryable(t, timer))
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
  /// use kwap::config::Std;
  /// use kwap::core::Core;
  /// use kwap::req::Req;
  ///
  /// let sock = UdpSocket::bind(("0.0.0.0", 8004)).unwrap();
  /// let mut core = Core::<Std>::new(Default::default(), sock);
  /// let id = core.ping("1.1.1.1", 5683);
  /// // core.poll_ping(id);
  /// ```
  pub fn ping(&mut self, host: impl AsRef<str>, port: u16) -> Result<(kwap_msg::Id, SocketAddr), Error<Cfg>> {
    let mut msg: config::Message<Cfg> = Req::<Cfg>::get(host.as_ref(), port, "").into();
    msg.token = kwap_msg::Token(Default::default());
    msg.opts = Default::default();
    msg.code = kwap_msg::Code::new(0, 0);

    let id = msg.id;
    msg.try_into_bytes::<ArrayVec<[u8; 13]>>()
       .map_err(Error::ToBytes)
       .tupled(|_| Ipv4Addr::from_str(host.as_ref()).map_err(|_| Error::HostInvalidIpAddress))
       .bind(|(bytes, host)| Self::send(&mut self.sock, SocketAddr::V4(SocketAddrV4::new(host, port)), bytes))
       .map(|addr| (id, addr))
  }

  /// Send a raw message down the wire to some remote host.
  ///
  /// You probably want [`send_req`](#method.send_req) or [`ping`](#method.ping) instead.
  fn send(sock: &mut Cfg::Socket,
          addr: SocketAddr,
          bytes: impl Array<Item = u8>)
          -> Result<SocketAddr, Error<Cfg>> {
    // TODO: uncouple from ipv4
    sock.connect(addr)
        .map_err(Error::SockError)
        .try_perform(|_| nb::block!(sock.send(&bytes)).map_err(Error::SockError))
        .map(|_| addr)
  }
}

impl<Cfg: Config> Eventer<Cfg> for Core<Cfg> {
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

  type Config = Alloc<crate::std::Clock, TubeSock>;

  #[test]
  fn eventer() {
    let req = Req::<Config>::get("0.0.0.0", 1234, "");
    let bytes = config::Message::<Config>::from(req).try_into_bytes::<ArrayVec<[u8; 1152]>>()
                                                    .unwrap();
    let mut client = Core::<Config>::behaviorless(crate::std::Clock::new(), TubeSock::new());

    fn on_err(_: &mut Core<Config>, e: &mut Event<Config>) {
      panic!("{:?}", e)
    }

    static mut CALLS: usize = 0;
    fn on_dgram(_: &mut Core<Config>, _: &mut Event<Config>) {
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
    type Msg = config::Message<Config>;

    let mut client = Core::<Config>::new(crate::std::Clock::new(), TubeSock::new());
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
    client.poll_ping(id, addr.into()).unwrap();
  }

  #[test]
  fn client_flow() {
    type Msg = config::Message<Config>;

    let req = Req::<Config>::get("0.0.0.0", 1234, "");
    let token = req.msg.token;
    let resp = Resp::<Config>::for_request(req);
    let bytes = Msg::from(resp).try_into_bytes::<Vec<u8>>().unwrap();

    let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 1234);
    let mut client = Core::<Config>::new(crate::std::Clock::new(),
                                         TubeSock::init(addr.clone().into(), bytes.clone()));

    let rep = client.poll_resp(token, addr.into()).unwrap();
    assert_eq!(bytes, Msg::from(rep).try_into_bytes::<Vec<u8>>().unwrap());
  }
}
