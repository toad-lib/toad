use core::mem;
use core::str::FromStr;

use embedded_time::duration::Milliseconds;
use embedded_time::Clock;
use kwap_common::prelude::*;
use kwap_msg::{Id, Token, TryFromBytes, TryIntoBytes, Type};
use no_std_net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use tinyvec::ArrayVec;

mod error;
#[doc(inline)]
pub use error::*;

use crate::net::{Addrd, Socket};
use crate::platform::{self, Platform, Retryable};
use crate::req::Req;
use crate::resp::Resp;
use crate::retry::RetryTimer;
use crate::time::Stamped;
use crate::todo::{Code, CodeKind};

// TODO(#81):
//   support environment variables:
//   - ACK_TIMEOUT
//   - ACK_RANDOM_FACTOR
//   - MAX_RETRANSMIT
//   - NSTART
//   - DEFAULT_LEISURE
//   - PROBING

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
pub struct Core<P: Platform> {
  /// See [`Core.next_token`] for info on our implementation of token generation.
  token_seed: u16,
  /// Map<SocketAddr, [Stamped<Id>]>
  msg_ids: P::MessageIdHistoryBySocket,
  largest_msg_id_seen: Option<u16>,
  /// Map<SocketAddr, [Stamped<Token>]>
  msg_tokens: P::MessageTokenHistoryBySocket,
  /// Networking socket that the CoAP runtime uses
  sock: P::Socket,
  /// Clock used for timing
  clock: P::Clock,
  /// Received responses
  resps: Buffer<Addrd<Resp<P>>, 16>,
  /// Queue of messages to send whose receipt we do not need to guarantee (NON, ACK)
  fling_q: Buffer<Addrd<platform::Message<P>>, 16>,
  /// Queue of confirmable messages that have not been ACKed and need to be sent again
  retry_q: Buffer<Retryable<P, Addrd<platform::Message<P>>>, 16>,
}

impl<P: Platform> Core<P> {
  /// Creates a new Core with the default runtime behavior
  pub fn new(clock: P::Clock, sock: P::Socket) -> Self {
    Self { token_seed: 0,
           sock,
           clock,
           msg_ids: Default::default(),
           largest_msg_id_seen: None,
           msg_tokens: Default::default(),
           resps: Default::default(),
           fling_q: Default::default(),
           retry_q: Default::default() }
  }

  fn seen_id(&mut self, id: Addrd<Id>) {
    if !self.msg_ids.has(&id.addr()) {
      self.msg_ids.insert(id.addr(), Default::default()).unwrap();
    }

    let ids_in_map = self.msg_ids.get_mut(&id.addr()).unwrap();

    let mut ids = P::MessageIdHistory::default();
    mem::swap(ids_in_map, &mut ids);

    let (mut ids, largest) = ids.into_iter()
                                .fold((P::MessageIdHistory::default(), None), |(mut ids, largest), id| {
                                  ids.push(id);
                                  (ids, Some(largest.filter(|large| *large > id.data().0).unwrap_or(id.data().0)))
                                });

    self.largest_msg_id_seen = largest.or(Some(id.data().0));

    ids.push(Stamped::new(&self.clock, *id.data()).unwrap());

    mem::swap(ids_in_map, &mut ids);
  }

  fn seen_token(&mut self, token: Addrd<Token>) {
    if !self.msg_tokens.has(&token.addr()) {
      self.msg_tokens.insert(token.addr(), Default::default()).unwrap();
    }

    let tokens_in_map = self.msg_tokens.get_mut(&token.addr()).unwrap();

    let mut tokens = P::MessageTokenHistory::default();
    mem::swap(tokens_in_map, &mut tokens);

    tokens = tokens.into_iter()
                   .filter(|token_b| token_b.data() != token.data())
                   .collect();
    tokens.push(Stamped::new(&self.clock, *token.data()).unwrap());

    mem::swap(tokens_in_map, &mut tokens);
  }

  fn next_id(&mut self, addr: SocketAddr) -> Id {
    // TODO: expiry

    let new = match self.largest_msg_id_seen {
      | Some(id) => Id(id + 1),
      | None => {
        // TODO: Random starting point?
        Id(0)
      },
    };

    self.seen_id(Addrd(new, addr));
    new
  }

  /// Token Generation
  ///
  /// First, we smoosh together the 2-byte token_seed
  /// and an 8-byte timestamp obtained from the system clock
  ///
  /// Core.token_seed
  /// ||
  /// xx xxxxxxxx
  ///    |      |
  ///    timestamp
  ///
  /// Then apply the BLAKE2 hashing algorithm to have an opaque 8 byte token.
  ///
  /// token_seed may be 0, a random integer, or a machine identifier.
  /// The purpose of the seed is to prevent someone with visibility into
  /// unencrypted CoAP traffic from guessing identifiers.
  fn next_token(&mut self, addr: SocketAddr) -> Token {
    // TODO: expiry

    let now_millis: Milliseconds<u64> = self.clock.try_now().unwrap().duration_since_epoch().try_into().unwrap();
    let now_millis: u64 = now_millis.0;
    let bytes = {
      // TODO: probably a better way to do this
      let ([a, b], [c, d, e, f, g, h, i, j]) = (self.token_seed.to_be_bytes(), now_millis.to_be_bytes());
      [a, b, c, d, e, f, g, h, i, j]
    };

    let token = crate::todo::Token::opaque(&bytes);
    self.seen_token(Addrd(token, addr));

    token
  }

  fn tick(&mut self) -> nb::Result<Option<Addrd<crate::net::Dgram>>, Error<P>> {
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

  fn retryable<T>(&self, when: When, t: T) -> Result<Retryable<P, T>, Error<P>> {
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
  pub fn store_resp(&mut self, resp: Addrd<Resp<P>>) -> () {
    if let Some(resp) = self.resps.try_push(Some(resp)) {
      // arrayvec is full, remove nones
      self.resps = self.resps.iter_mut().filter_map(|o| o.take()).map(Some).collect();

      // panic if we're still full
      self.resps.push(resp);
    }
  }

  /// Listens for incoming ACKs and removes any matching CON messages queued for retry.
  pub fn process_acks(&mut self, msg: &Addrd<platform::Message<P>>) {
    match msg.data().ty {
      | Type::Ack | Type::Reset => {
        let (id, addr) = (msg.data().id, msg.addr());
        let ix = self.retry_q
                     .iter()
                     .filter_map(Option::as_ref)
                     .enumerate()
                     .find(|(_, Retryable(Addrd(con, con_addr), _))| *con_addr == addr && con.id == id)
                     .map(|(ix, _)| ix);

        if let Some(ix) = ix {
          self.retry_q.remove(ix);
        } else {
          // TODO(#76): we got an ACK for a message we don't know about. What do we do?
        }
      },
      // TODO: ACK incoming CON responses
      | _ => (),
    }
  }

  /// Poll for a response to a sent request
  ///
  /// # Example
  /// See `./examples/client.rs`
  pub fn poll_resp(&mut self, token: kwap_msg::Token, sock: SocketAddr) -> nb::Result<Resp<P>, Error<P>> {
    self.tick().bind(|_| {
                 self.try_get_resp(token, sock)
                     .map_err(|nb_err| nb_err.map(What::SockError).map(|what| When::Polling.what(what)))
               })
  }

  /// Poll for an incoming request
  pub fn poll_req(&mut self) -> nb::Result<Addrd<Req<P>>, Error<P>> {
    let when = When::Polling;

    self.tick()
        .bind(|dgram| dgram.ok_or(nb::Error::WouldBlock))
        .bind(|Addrd(dgram, addr)| {
          platform::Message::<P>::try_from_bytes(dgram).map_err(What::FromBytes)
                                                       .map_err(|what| when.what(what))
                                                       .map_err(nb::Error::Other)
                                                       .map(|msg| Addrd(msg, addr))
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
  pub fn poll_ping(&mut self, req_id: kwap_msg::Id, addr: SocketAddr) -> nb::Result<(), Error<P>> {
    self.tick().bind(|_| {
                 self.check_ping(req_id, addr)
                     .map_err(|nb_err| nb_err.map(What::SockError).map(|what| When::Polling.what(what)))
               })
  }

  pub(super) fn dgram_recvd(&mut self, when: error::When, dgram: Addrd<crate::net::Dgram>) -> Result<(), Error<P>> {
    platform::Message::<P>::try_from_bytes(dgram.data()).map(|msg| dgram.map(|_| msg))
                                                        .map_err(What::FromBytes)
                                                        .map_err(|what| when.what(what))
                                                        .map(|msg| self.msg_recvd(msg))
  }

  fn msg_recvd(&mut self, msg: Addrd<platform::Message<P>>) -> () {
    self.seen_id(msg.as_ref().map(|msg| msg.id));
    self.seen_token(msg.as_ref().map(|msg| msg.token));

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
                  -> nb::Result<Resp<P>, <<P as Platform>::Socket as Socket>::Error> {
    let resp_matches = |o: &Option<Addrd<Resp<P>>>| {
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
          | mut o @ Some(_) if resp_matches(&o) => Option::take(&mut o).map(|Addrd(resp, _)| resp),
          | _ => None,
        })
        .ok_or(nb::Error::WouldBlock)
  }

  fn check_ping(&mut self,
                req_id: kwap_msg::Id,
                addr: SocketAddr)
                -> nb::Result<(), <<P as Platform>::Socket as Socket>::Error> {
    let still_qd = self.retry_q
                       .iter()
                       .filter_map(|o| o.as_ref())
                       .any(|Retryable(Addrd(con, con_addr), _)| con.id == req_id && addr == *con_addr);

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
  pub fn send_flings(&mut self) -> Result<(), Error<P>> {
    self.fling_q
        .iter_mut()
        .filter_map(Option::take)
        .try_for_each(|Addrd(msg, addr)| {
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
  pub fn send_retrys(&mut self) -> Result<(), Error<P>> {
    use crate::retry::YouShould;

    self.retry_q
        .iter_mut()
        .filter_map(|o| o.as_mut())
        .try_for_each(|Retryable(Addrd(msg, addr), retry)| {
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
  /// use kwap::core::Core;
  /// use kwap::platform::Std;
  /// use kwap::req::Req;
  ///
  /// let sock = UdpSocket::bind(("0.0.0.0", 8002)).unwrap();
  /// let mut core = Core::<Std>::new(Default::default(), sock);
  /// core.send_req(Req::<Std>::get("1.1.1.1", 5683, "/hello"));
  /// ```
  pub fn send_req(&mut self, mut req: Req<P>) -> Result<(kwap_msg::Token, SocketAddr), Error<P>> {
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

    let when = When::None;

    core::str::from_utf8(&host).map_err(|err| when.what(What::HostInvalidUtf8(err)))
                               .bind(|host| Ipv4Addr::from_str(host).map_err(|_| when.what(What::HostInvalidIpAddress)))
                               .map(|host| SocketAddr::V4(SocketAddrV4::new(host, port)))
                               .map(|host| {
                                 if req.id.is_none() {
                                   req.set_msg_id(self.next_id(host));
                                 }
                                 if req.token.is_none() {
                                   req.set_msg_token(self.next_token(host));
                                 }

                                 (req.msg_token(), host)
                               })
                               .and_then(|(token, addr)| {
                                 let msg = platform::Message::<P>::from(req);
                                 // TODO: avoid this clone?
                                 self.retryable(when, Addrd(msg.clone(), addr))
                                     .map(|msg| self.retry_q.push(Some(msg)))
                                     .map(|_| (token, addr, msg))
                               })
                               .and_then(|(token, addr, msg)| {
                                 msg.try_into_bytes::<ArrayVec<[u8; 1152]>>()
                                    .map_err(|err| when.what(What::ToBytes(err)))
                                    .map(|bytes| (token, addr, bytes))
                               })
                               .bind(|(token, addr, bytes)| {
                                 Self::send(when, &mut self.sock, addr, bytes).map(|addr| (token, addr))
                               })
  }

  /// Send a message to a remote socket
  pub fn send_msg(&mut self, msg: Addrd<platform::Message<P>>) -> Result<(), Error<P>> {
    let addr = msg.addr();
    let when = When::SendingMessage(Some(msg.addr()), msg.data().id, msg.data().token);

    msg.unwrap()
       .try_into_bytes::<ArrayVec<[u8; 1152]>>()
       .map_err(What::<P>::ToBytes)
       .map_err(|what| when.what(what))
       .bind(|bytes| Self::send(when, &mut self.sock, addr, bytes))
       .map(|_| ())
  }

  /// Send a raw message down the wire to some remote host.
  ///
  /// You probably want [`send_req`](#method.send_req) or [`ping`](#method.ping) instead.
  pub(crate) fn send(when: When,
                     sock: &mut P::Socket,
                     addr: SocketAddr,
                     bytes: impl Array<Item = u8>)
                     -> Result<SocketAddr, Error<P>> {
    // TODO(#77): support ipv6
    nb::block!(sock.send(Addrd(&bytes, addr))).map_err(|err| when.what(What::SockError(err)))
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
  /// use kwap::core::Core;
  /// use kwap::platform::Std;
  /// use kwap::req::Req;
  ///
  /// let sock = UdpSocket::bind(("0.0.0.0", 8004)).unwrap();
  /// let mut core = Core::<Std>::new(Default::default(), sock);
  /// let id = core.ping("1.1.1.1", 5683);
  /// // core.poll_ping(id);
  /// ```
  pub fn ping(&mut self, host: impl AsRef<str>, port: u16) -> Result<(kwap_msg::Id, SocketAddr), Error<P>> {
    let when = When::None;

    Ipv4Addr::from_str(host.as_ref()).map_err(|_| when.what(What::HostInvalidIpAddress))
                                     .map(|host| SocketAddr::V4(SocketAddrV4::new(host, port)))
                                     .map(|addr| (addr, self.next_id(addr)))
                                     .and_then(|(addr, id)| {
                                       let mut req = Req::<P>::get(host, port, "");
                                       req.set_msg_id(id);
                                       req.set_msg_token(Token(Default::default()));

                                       let mut msg: platform::Message<P> = req.into();
                                       msg.opts = Default::default();
                                       msg.code = kwap_msg::Code::new(0, 0);

                                       msg.try_into_bytes::<ArrayVec<[u8; 13]>>()
                                          .map_err(|err| when.what(What::ToBytes(err)))
                                          .map(|bytes| (addr, id, bytes))
                                     })
                                     .bind(|(addr, id, bytes)| {
                                       Self::send(When::None, &mut self.sock, addr, bytes).map(|addr| (id, addr))
                                     })
  }
}

#[cfg(test)]
mod tests {
  use kwap_msg::TryIntoBytes;
  use tinyvec::ArrayVec;

  use super::*;
  use crate::platform;
  use crate::platform::Alloc;
  use crate::req::Req;
  use crate::test::SockMock;

  type Config = Alloc<crate::std::Clock, SockMock>;

  #[test]
  fn ping() {
    type Msg = platform::Message<Config>;

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
    type Msg = platform::Message<Config>;

    let req = Req::<Config>::get("0.0.0.0", 1234, "");
    let token = req.msg.token;
    let resp = Resp::<Config>::for_request(&req).unwrap();
    let bytes = Msg::from(resp).try_into_bytes::<Vec<u8>>().unwrap();

    let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 1234);
    let sock = SockMock::new();
    sock.rx.lock().unwrap().push(Addrd(bytes.clone(), addr.into()));
    let mut client = Core::<Config>::new(crate::std::Clock::new(), sock);

    let rep = client.poll_resp(token, addr.into()).unwrap();
    assert_eq!(bytes, Msg::from(rep).try_into_bytes::<Vec<u8>>().unwrap());
  }
}
