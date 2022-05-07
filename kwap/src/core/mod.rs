use core::str::FromStr;

use embedded_time::Clock;
use kwap_common::prelude::*;
use kwap_msg::{TryFromBytes, TryIntoBytes, Type};
use no_std_net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use tinyvec::ArrayVec;

mod error;
#[doc(inline)]
pub use error::*;

use crate::config::{self, Config, Retryable};
use crate::req::Req;
use crate::resp::Resp;
use crate::retry::RetryTimer;
use crate::socket::{Addressed, Socket};
use crate::todo::{Code, CodeKind, Message};

// TODO(#81):
//   support environment variables:
//   - ACK_TIMEOUT
//   - ACK_RANDOM_FACTOR
//   - MAX_RETRANSMIT
//   - NSTART
//   - DEFAULT_LEISURE
//   - PROBING

// TODO(#86): handle ping requests silently

// Option for these collections provides a Default implementation,
// which is required by ArrayVec.
//
// This also allows us efficiently take owned responses from the collection without reindexing the other elements.
type Buffer<T, const N: usize> = ArrayVec<[Option<T>; N]>;

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
  /// Received responses
  resps: Buffer<Addressed<Resp<Cfg>>, 16>,
  /// Queue of messages to send whose receipt we do not need to guarantee (NON, ACK)
  fling_q: Buffer<Addressed<config::Message<Cfg>>, 16>,
  /// Queue of confirmable messages that have not been ACKed and need to be sent again
  retry_q: Buffer<Retryable<Cfg, Addressed<config::Message<Cfg>>>, 16>,
}

impl<Cfg: Config> Core<Cfg> {
  /// Creates a new Core with the default runtime behavior
  pub fn new(clock: Cfg::Clock, sock: Cfg::Socket) -> Self {
    Self { sock,
           clock,
           resps: Default::default(),
           fling_q: Default::default(),
           retry_q: Default::default() }
  }

  fn tick(&mut self) -> nb::Result<Option<Addressed<crate::socket::Dgram>>, Error<Cfg>> {
    let when = When::Polling;

    self.sock
        .poll()
        .map_err(|e| when.what(What::SockError(e)))
        // TODO: This is a /bad/ copy.
        .try_perform(|polled| polled.map(|ref dgram| self.dgram_recvd(when, *dgram)).unwrap_or(Ok(())))
        .try_perform(|_| self.send_flings())
        .try_perform(|_| self.send_retrys())
        .map_err(nb::Error::Other)
  }

  fn retryable<T>(&self, when: When, t: T) -> Result<Retryable<Cfg, T>, Error<Cfg>> {
    self.clock
        .try_now()
        .map(|now| {
          RetryTimer::new(now,
                          crate::retry::Strategy::Exponential(embedded_time::duration::Milliseconds(100)),
                          crate::retry::Attempts(5))
        })
        .map_err(|_| when.what(What::ClockError))
        .map(|timer| Retryable(t, timer))
  }

  /// Listens for RecvResp events and stores them on the runtime struct
  ///
  /// # Panics
  /// panics when response tracking limit reached (e.g. 64 requests were sent and we haven't polled for a response of a single one)
  pub fn store_resp(&mut self, resp: Addressed<Resp<Cfg>>) -> () {
    if let Some(resp) = self.resps.try_push(Some(resp)) {
      // arrayvec is full, remove nones
      self.resps = self.resps.iter_mut().filter_map(|o| o.take()).map(Some).collect();

      // panic if we're still full
      self.resps.push(resp);
    }
  }

  /// Listens for incoming CONfirmable messages and places them on a queue to reply to with ACKs.
  ///
  /// These ACKs are processed whenever the socket is polled (e.g. [`poll_resp`](#method.poll_resp))
  ///
  /// # Panics
  /// panics when msg storage limit reached (e.g. we receive >16 CON requests and have not acked any)
  pub fn ack(&mut self, resp: Addressed<Resp<Cfg>>) {
    if resp.data().msg_type() == kwap_msg::Type::Con {
      let ack_id = crate::generate_id();
      let ack = resp.map(|resp| resp.msg.ack(ack_id));

      self.fling_q.push(Some(ack));
    }
  }

  /// Listens for incoming ACKs and removes any matching CON messages queued for retry.
  pub fn process_acks(&mut self, msg: &Addressed<config::Message<Cfg>>) {
    if msg.data().ty == Type::Ack {
      let (id, addr) = (msg.data().id, msg.addr());
      let ix = self.retry_q
                   .iter()
                   .filter_map(Option::as_ref)
                   .enumerate()
                   .find(|(_, Retryable(Addressed(con, con_addr), _))| *con_addr == addr && con.id == id)
                   .map(|(ix, _)| ix);

      if let Some(ix) = ix {
        self.retry_q.remove(ix);
      } else {
        // TODO(#76): we got an ACK for a message we don't know about. What do we do?
      }
    }
  }

  /// Poll for a response to a sent request
  ///
  /// # Example
  /// See `./examples/client.rs`
  pub fn poll_resp(&mut self, token: kwap_msg::Token, sock: SocketAddr) -> nb::Result<Resp<Cfg>, Error<Cfg>> {
    self.tick().bind(|_| {
                 self.try_get_resp(token, sock)
                     .map_err(|nb_err| nb_err.map(What::SockError).map(|what| When::Polling.what(what)))
               })
  }

  /// Poll for an incoming request
  pub fn poll_req(&mut self) -> nb::Result<Addressed<Req<Cfg>>, Error<Cfg>> {
    let when = When::Polling;

    self.tick()
        .bind(|dgram| dgram.ok_or(nb::Error::WouldBlock))
        .bind(|Addressed(dgram, addr)| {
          config::Message::<Cfg>::try_from_bytes(dgram).map_err(What::FromBytes)
                                                       .map_err(|what| when.what(what))
                                                       .map_err(nb::Error::Other)
                                                       .map(|msg| Addressed(msg, addr))
        })
        .map(|addrd| addrd.map(Req::from))
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
    self.tick().bind(|_| {
                 self.check_ping(req_id, addr)
                     .map_err(|nb_err| nb_err.map(What::SockError).map(|what| When::Polling.what(what)))
               })
  }

  pub(super) fn dgram_recvd(&mut self,
                            when: error::When,
                            dgram: Addressed<crate::socket::Dgram>)
                            -> Result<(), Error<Cfg>> {
    config::Message::<Cfg>::try_from_bytes(dgram.data()).map(|msg| dgram.map(|_| msg))
                                                        .map_err(What::FromBytes)
                                                        .map_err(|what| when.what(what))
                                                        .map(|msg| self.msg_recvd(msg))
  }

  fn msg_recvd(&mut self, msg: Addressed<config::Message<Cfg>>) -> () {
    self.process_acks(&msg);

    if msg.data().code.kind() == CodeKind::Response {
      // TODO(#84):
      //   I don't think we need to store responses and whatnot at all now
      //   that the event system is dead
      self.store_resp(msg.map(Into::into));
    }
  }

  fn try_get_resp(&mut self,
                  token: kwap_msg::Token,
                  sock: SocketAddr)
                  -> nb::Result<Resp<Cfg>, <<Cfg as Config>::Socket as Socket>::Error> {
    let resp_matches = |o: &Option<Addressed<Resp<Cfg>>>| {
      o.as_ref()
       .map(|rep| {
         rep.as_ref()
            .map_with_addr(|rep, addr| rep.msg.token == token && addr == sock)
            .unwrap()
       })
       .unwrap_or(false)
    };

    self.resps
        .iter_mut()
        .find_map(|rep| match rep {
          #[allow(clippy::needless_borrow)]
          | mut o @ Some(_) if resp_matches(&o) => Option::take(&mut o).map(|Addressed(resp, _)| resp),
          | _ => None,
        })
        .ok_or(nb::Error::WouldBlock)
  }

  fn check_ping(&mut self,
                req_id: kwap_msg::Id,
                addr: SocketAddr)
                -> nb::Result<(), <<Cfg as Config>::Socket as Socket>::Error> {
    let still_qd = self.retry_q
                       .iter()
                       .filter_map(|o| o.as_ref())
                       .any(|Retryable(Addressed(con, con_addr), _)| con.id == req_id && addr == *con_addr);

    if still_qd {
      Err(nb::Error::WouldBlock)
    } else {
      Ok(())
    }
  }

  /// Process all the queued outbound messages that **we will send once and never retry**.
  ///
  /// By default, we do not consider outbound NON-confirmable requests "flings" because
  /// we **do** want to retransmit them in the case that it is lost & the server will respond to it.
  ///
  /// We treat outbound NON and CON requests the same way in the core so that
  /// we can allow for users to choose whether a NON that was transmitted multiple times
  /// without a response is an error condition or good enough.
  pub fn send_flings(&mut self) -> Result<(), Error<Cfg>> {
    self.fling_q
        .iter_mut()
        .filter_map(Option::take)
        .try_for_each(|Addressed(msg, addr)| {
          let (id, token) = (msg.id, msg.token);
          let when = When::SendingMessage(Some(addr), id, token);

          msg.try_into_bytes::<ArrayVec<[u8; 1152]>>()
             .map_err(|e| when.what(What::ToBytes(e)))
             .bind(|bytes| Self::send(when, &mut self.sock, addr, bytes))
             .map(|_| ())
        })
  }

  /// Process all the queued outbound messages **that we may send multiple times based on the response behavior**.
  ///
  /// The expectation is that when these messages are Acked, an event handler
  /// will remove them from storage.
  pub fn send_retrys(&mut self) -> Result<(), Error<Cfg>> {
    use crate::retry::YouShould;

    self.retry_q
        .iter_mut()
        .filter_map(|o| o.as_mut())
        .try_for_each(|Retryable(Addressed(msg, addr), retry)| {
          let (id, token) = (msg.id, msg.token);
          let when = When::SendingMessage(Some(*addr), id, token);

          msg.clone()
             .try_into_bytes::<ArrayVec<[u8; 1152]>>()
             .map_err(|err| when.what(What::ToBytes(err)))
             .tupled(|_| {
               self.clock
                   .try_now()
                   .map_err(|_| when.what(What::ClockError))
                   .map(|now| retry.what_should_i_do(now))
             })
             .bind(|(bytes, should)| match should {
               | Ok(YouShould::Retry) => Self::send(when, &mut self.sock, *addr, bytes).map(|_| ()),
               | Ok(YouShould::Cry) => Err(when.what(What::MessageNeverAcked)),
               | Err(nb::Error::WouldBlock) => Ok(()),
               | _ => unreachable!(),
             })
        })
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
    let when = When::SendingMessage(None, msg.id, msg.token);

    core::str::from_utf8(&host).map_err(|err| when.what(What::HostInvalidUtf8(err)))
                               .bind(|host| Ipv4Addr::from_str(host).map_err(|_| when.what(What::HostInvalidIpAddress)))
                               .map(|host| SocketAddr::V4(SocketAddrV4::new(host, port)))
                               .try_perform(|addr| {
                                 let t = Addressed(msg.clone(), *addr);
                                 self.retryable(when, t).map(|bam| self.retry_q.push(Some(bam)))
                               })
                               .tupled(|_| {
                                 msg.try_into_bytes::<ArrayVec<[u8; 1152]>>()
                                    .map_err(|err| when.what(What::ToBytes(err)))
                               })
                               .bind(|(addr, bytes)| Self::send(when, &mut self.sock, addr, bytes))
                               .map(|addr| (token, addr))
  }

  /// Send a message to a remote socket
  pub fn send_msg(&mut self, msg: Addressed<config::Message<Cfg>>) -> Result<(), Error<Cfg>> {
    let addr = msg.addr();
    let when = When::SendingMessage(Some(msg.addr()), msg.data().id, msg.data().token);
    msg.unwrap()
       .try_into_bytes::<ArrayVec<[u8; 1152]>>()
       .map_err(What::<Cfg>::ToBytes)
       .map_err(|what| when.what(what))
       .bind(|bytes| Self::send(when, &mut self.sock, addr, bytes))
       .map(|_| ())
  }

  /// Send a raw message down the wire to some remote host.
  ///
  /// You probably want [`send_req`](#method.send_req) or [`ping`](#method.ping) instead.
  pub(crate) fn send(when: When,
                     sock: &mut Cfg::Socket,
                     addr: SocketAddr,
                     bytes: impl Array<Item = u8>)
                     -> Result<SocketAddr, Error<Cfg>> {
    // TODO(#77): support ipv6
    nb::block!(sock.send(Addressed(&bytes, addr))).map_err(|err| when.what(What::SockError(err)))
                                                  .map(|_| addr)
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

    let (id, token) = (msg.id, msg.token);
    let when = When::SendingMessage(None, id, token);

    let bytes = msg.try_into_bytes::<ArrayVec<[u8; 13]>>()
                   .map_err(|err| when.what(What::ToBytes(err)));

    let host = Ipv4Addr::from_str(host.as_ref()).map_err(|_| when.what(What::HostInvalidIpAddress));

    Result::two(bytes, host).map(|(bytes, host)| (bytes, SocketAddr::V4(SocketAddrV4::new(host, port))))
                            .bind(|(bytes, host)| {
                              Self::send(When::SendingMessage(Some(host), id, token), &mut self.sock, host, bytes)
                            })
                            .map(|addr| (id, addr))
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
  use crate::test::SockMock;

  type Config = Alloc<crate::std::Clock, SockMock>;

  #[test]
  fn ping() {
    type Msg = config::Message<Config>;

    let mut client = Core::<Config>::new(crate::std::Clock::new(), SockMock::new());
    let (id, addr) = client.ping("0.0.0.0", 5632).unwrap();

    let resp = Msg { id,
                     token: kwap_msg::Token(Default::default()),
                     code: kwap_msg::Code::new(0, 0),
                     ver: Default::default(),
                     ty: kwap_msg::Type::Reset,
                     payload: kwap_msg::Payload(Default::default()),
                     opts: Default::default() };

    let _bytes = resp.try_into_bytes::<ArrayVec<[u8; 1152]>>().unwrap();

    // client.fire(Event::RecvDgram(Some((bytes, addr)))).unwrap();
    client.poll_ping(id, addr).unwrap();
  }

  #[test]
  fn client_flow() {
    type Msg = config::Message<Config>;

    let req = Req::<Config>::get("0.0.0.0", 1234, "");
    let token = req.msg.token;
    let resp = Resp::<Config>::for_request(req);
    let bytes = Msg::from(resp).try_into_bytes::<Vec<u8>>().unwrap();

    let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 1234);
    let sock = SockMock::new();
    sock.rx.lock().unwrap().push(Addressed(bytes.clone(), addr.into()));
    let mut client = Core::<Config>::new(crate::std::Clock::new(), sock);

    let rep = client.poll_resp(token, addr.into()).unwrap();
    assert_eq!(bytes, Msg::from(rep).try_into_bytes::<Vec<u8>>().unwrap());
  }
}
