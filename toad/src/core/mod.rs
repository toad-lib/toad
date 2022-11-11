use core::mem;

use embedded_time::duration::Milliseconds;
use embedded_time::{Clock, Instant};
use no_std_net::{IpAddr, SocketAddr};
use rand::{Rng, SeedableRng};
use tinyvec::ArrayVec;
use toad_common::*;
use toad_msg::{CodeKind, Id, Token, TryFromBytes, TryIntoBytes, Type};

mod error;
#[doc(inline)]
pub use error::*;

use crate::config::Config;
use crate::logging;
use crate::net::{Addrd, Socket};
use crate::platform::{self, Platform, Retryable};
use crate::req::Req;
use crate::resp::Resp;
use crate::retry::RetryTimer;
use crate::time::Stamped;
use crate::todo::Capacity;

/// DTLS mode
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Secure {
  /// Opt in to DTLS, if platform supports it
  IfSupported,
  /// Explicitly opt out of DTLS
  #[allow(dead_code)]
  No,
}

// Option for these collections provides a Default implementation,
// which is required by ArrayVec.
//
// This also allows us efficiently take owned responses from the collection without reindexing the other elements.
type Buffer<T, const N: usize> = ArrayVec<[Option<T>; N]>;

/// A CoAP request/response runtime that drives client- and server-side behavior.
#[allow(missing_debug_implementations)]
pub struct Core<P: Platform> {
  /// Map<SocketAddr, Array<Stamped<Id>>>
  msg_ids: P::MessageIdHistoryBySocket,
  /// Map<SocketAddr, Array<Stamped<Token>>>
  msg_tokens: P::MessageTokenHistoryBySocket,

  /// Received responses
  resps: Buffer<Addrd<Resp<P>>, 16>,

  /// Queue of messages to send whose receipt we do not need to guarantee (NON, ACK)
  fling_q: Buffer<Addrd<platform::Message<P>>, 16>,

  /// Queue of confirmable messages that have not been ACKed and need to be sent again
  retry_q: Buffer<Retryable<P, Addrd<platform::Message<P>>>, 16>,

  sock: P::Socket,
  pub(crate) clock: P::Clock,

  largest_msg_id_seen: Option<u16>,
  rand: rand_chacha::ChaCha8Rng,
  config: Config,
}

impl<P: Platform> Core<P> {
  /// Creates a new Core with the default runtime behavior
  pub fn new(clock: P::Clock, sock: P::Socket) -> Self {
    Self::new_config(Config::default(), clock, sock)
  }

  /// Create a new core with custom runtime behavior
  pub fn new_config(config: Config, clock: P::Clock, sock: P::Socket) -> Self {
    Self { config: config.into(),
           rand: rand_chacha::ChaCha8Rng::seed_from_u64(0),
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
    let now = self.clock.try_now().unwrap();
    let millis_since = |other: &Instant<P::Clock>| {
      now.checked_duration_since(other)
         .and_then(|dur| Milliseconds::<u64>::try_from(dur).ok())
         .unwrap()
         .0
    };

    if !self.msg_ids.has(&id.addr()) {
      self.msg_ids.insert(id.addr(), Default::default()).unwrap();
    }

    let ids_in_map = self.msg_ids.get_mut(&id.addr()).unwrap();

    let mut ids = P::MessageIdHistory::default();
    mem::swap(ids_in_map, &mut ids);

    let (mut ids, largest) =
      ids.into_iter()
         .filter(|id| millis_since(&id.time()) < self.config.exchange_lifetime_millis() as u64)
         .fold((P::MessageIdHistory::default(), None),
               |(mut ids, largest), id| {
                 ids.push(id);
                 (ids,
                  Some(largest.filter(|large| *large > id.data().0)
                              .unwrap_or(id.data().0)))
               });

    self.largest_msg_id_seen = largest.or_else(|| Some(id.data().0));

    ids.push(Stamped::new(&self.clock, *id.data()).unwrap());
    let ids_cap = ids.capacity_pct();

    mem::swap(ids_in_map, &mut ids);

    log::trace!("stored new id ({}% capacity for addr, {}% total)",
                ids_cap.unwrap_or_default(),
                self.msg_ids.capacity_pct().unwrap_or_default());
  }

  fn seen_token(&mut self, token: Addrd<Token>) {
    let now = self.clock.try_now().unwrap();
    let millis_since = |other: &Instant<P::Clock>| {
      now.checked_duration_since(other)
         .and_then(|dur| Milliseconds::<u64>::try_from(dur).ok())
         .unwrap()
         .0
    };

    if !self.msg_tokens.has(&token.addr()) {
      self.msg_tokens
          .insert(token.addr(), Default::default())
          .unwrap();
    }

    let tokens_in_map = self.msg_tokens.get_mut(&token.addr()).unwrap();

    let mut tokens = P::MessageTokenHistory::default();
    mem::swap(tokens_in_map, &mut tokens);

    tokens = tokens.into_iter()
                   .filter(|token_b| {
                     // if we've seen this token before, assume it's a retransmission
                     // in which case the old timestamp should be tossed out in favor
                     // of right now
                     token_b.data() != token.data()
                     && millis_since(&token_b.time())
                        < self.config.exchange_lifetime_millis() as u64
                   })
                   .collect();

    let tokens_cap = tokens.capacity_pct();

    tokens.push(Stamped::new(&self.clock, *token.data()).unwrap());

    mem::swap(tokens_in_map, &mut tokens);

    log::trace!("stored new token ({}% capacity for addr, {}% total)",
                tokens_cap.unwrap_or_default(),
                self.msg_tokens.capacity_pct().unwrap_or_default());
  }

  fn next_id(&mut self, addr: SocketAddr) -> Id {
    let new = match self.largest_msg_id_seen {
      | Some(id) => Id(id + 1),
      | None => Id(self.rand.gen_range(0..=255)),
    };

    self.seen_id(Addrd(new, addr));
    new
  }

  fn next_token(&mut self, addr: SocketAddr) -> Token {
    let now_millis: Milliseconds<u64> = self.clock
                                            .try_now()
                                            .unwrap()
                                            .duration_since_epoch()
                                            .try_into()
                                            .unwrap();
    let now_millis: u64 = now_millis.0;

    #[allow(clippy::many_single_char_names)]
    let bytes = {
      let ([a, b], [c, d, e, f, g, h, i, j]) =
        (self.config.msg.token_seed.to_be_bytes(), now_millis.to_be_bytes());
      [a, b, c, d, e, f, g, h, i, j]
    };

    let token = Token::opaque(&bytes);
    self.seen_token(Addrd(token, addr));

    token
  }

  fn tick(&mut self) -> nb::Result<Option<Addrd<crate::net::Dgram>>, Error<P>> {
    let when = When::Polling;

    self.sock
        .poll()
        .map_err(|e| when.what(What::SockError(e)))
        // TODO: This is a /bad/ copy.
        .try_perform(|polled| {
          polled.map(|ref dgram| self.dgram_recvd(when, *dgram))
                .unwrap_or(Ok(()))
        })
        .try_perform(|_| self.send_flings())
        .try_perform(|_| self.send_retrys())
        .map_err(nb::Error::Other)
  }

  fn retryable<T>(&self, when: When, t: T) -> Result<Retryable<P, T>, Error<P>> {
    self.clock
        .try_now()
        .map(|now| {
          RetryTimer::new(now,
                          self.config.msg.con.unacked_retry_strategy,
                          self.config.msg.con.max_attempts)
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
      self.resps = self.resps
                       .iter_mut()
                       .filter_map(|o| o.take())
                       .map(Some)
                       .collect();

      // panic if we're still full
      self.resps.push(resp);
    }
  }

  /// Listens for incoming ACKs and removes any matching CON messages queued for retry.
  pub fn process_acks(&mut self, msg: &Addrd<platform::Message<P>>) {
    match msg.data().ty {
      | Type::Ack | Type::Reset => {
        let (id, addr) = (msg.data().id, msg.addr());
        let ix =
          self.retry_q
              .iter()
              .filter_map(Option::as_ref)
              .enumerate()
              .find(|(_, Retryable(Addrd(con, con_addr), _))| *con_addr == addr && con.id == id)
              .map(|(ix, _)| ix);

        if let Some(ix) = ix {
          let msg: Retryable<P, Addrd<platform::Message<P>>> = self.retry_q.remove(ix).unwrap();
          log::trace!("{:?} was Acked", msg.unwrap().unwrap().id);
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
  pub fn poll_resp(&mut self,
                   token: toad_msg::Token,
                   sock: SocketAddr)
                   -> nb::Result<Resp<P>, Error<P>> {
    self.tick().bind(|_| {
                 self.try_get_resp(token, sock).map_err(|nb_err| {
                                                 nb_err.map(What::SockError)
                                                       .map(|what| When::Polling.what(what))
                                               })
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
  pub fn poll_ping(&mut self, req_id: toad_msg::Id, addr: SocketAddr) -> nb::Result<(), Error<P>> {
    self.tick().bind(|_| {
                 self.check_ping(req_id, addr).map_err(|nb_err| {
                                                nb_err.map(What::SockError)
                                                      .map(|what| When::Polling.what(what))
                                              })
               })
  }

  pub(super) fn dgram_recvd(&mut self,
                            when: error::When,
                            dgram: Addrd<crate::net::Dgram>)
                            -> Result<(), Error<P>> {
    log::trace!("recvd {}b <- {}", dgram.data().get_size(), dgram.addr());
    platform::Message::<P>::try_from_bytes(dgram.data()).map(|msg| dgram.map(|_| msg))
                                                        .map_err(What::FromBytes)
                                                        .map_err(|what| when.what(what))
                                                        .map(|msg| self.msg_recvd(msg))
  }

  fn msg_recvd(&mut self, msg: Addrd<platform::Message<P>>) -> () {
    log::trace!("recvd {} <- {}",
                logging::msg_summary::<P>(msg.data()).as_str(),
                msg.addr());

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
                  token: toad_msg::Token,
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
                req_id: toad_msg::Id,
                addr: SocketAddr)
                -> nb::Result<(), <<P as Platform>::Socket as Socket>::Error> {
    let still_qd =
      self.retry_q
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
        .try_for_each(|msg| {
          Self::send_msg_sock(&mut self.sock, msg, Secure::IfSupported).map(|_| ())
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
        .try_for_each(|Retryable(msg, retry)| {
          let when = When::None;

          self.clock
              .try_now()
              .map_err(|_| when.what(What::ClockError))
              .map(|now| retry.what_should_i_do(now))
              .bind(|should| match should {
                | Ok(YouShould::Retry) => {
                  Self::send_msg_sock(&mut self.sock, msg.clone(), Secure::IfSupported).map(|_| ())
                },
                | Ok(YouShould::Cry) => Err(when.what(What::MessageNeverAcked)),
                | Err(nb::Error::WouldBlock) => Ok(()),
                | _ => unreachable!(),
              })
        })
  }

  pub(crate) fn send_req(&mut self,
                         req: Req<P>,
                         secure: Secure)
                         -> Result<(toad_msg::Token, SocketAddr), Error<P>> {
    let port = req.get_option(7).expect("Uri-Port must be present");
    let port_bytes = port.value
                         .0
                         .iter()
                         .take(2)
                         .copied()
                         .collect::<ArrayVec<[u8; 2]>>();
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
                               .map(|host| host.parse::<IpAddr>().unwrap())
                               .map(|host| SocketAddr::new(host, port))
                               .bind(|host| self.send_addrd_req(Addrd(req, host), secure))
  }

  pub(crate) fn send_addrd_req(&mut self,
                               mut req: Addrd<Req<P>>,
                               secure: Secure)
                               -> Result<(toad_msg::Token, SocketAddr), Error<P>> {
    let addr = req.addr();

    if req.data().id.is_none() {
      req.as_mut().set_msg_id(self.next_id(addr));
    }

    if req.data().token.is_none() {
      req.as_mut().set_msg_token(self.next_token(addr));
    }

    let msg = req.map(platform::Message::<P>::from);
    let token = msg.data().token;

    // TODO: avoid this clone?
    self.retryable(When::None, msg.clone())
        .map(|msg| {
          if msg.0.data().ty == Type::Con {
            self.retry_q.push(Some(msg))
          }
        })
        .bind(|_| Self::send_msg_sock(&mut self.sock, msg, secure))
        .map(|()| (token, addr))
  }

  /// Send a message to a remote socket
  pub(crate) fn send_msg(&mut self,
                         msg: Addrd<platform::Message<P>>,
                         secure: Secure)
                         -> Result<(), Error<P>> {
    Self::send_msg_sock(&mut self.sock, msg, secure)
  }

  fn send_msg_sock(sock: &mut P::Socket,
                   msg: Addrd<platform::Message<P>>,
                   secure: Secure)
                   -> Result<(), Error<P>> {
    let addr = msg.addr();
    let when = When::None;

    log::trace!("sending {} -> {}",
                logging::msg_summary::<P>(msg.data()).as_str(),
                msg.addr());

    msg.unwrap()
       .try_into_bytes::<ArrayVec<[u8; 1152]>>()
       .map_err(What::<P>::ToBytes)
       .map_err(|what| when.what(what))
       .bind(|bytes| Self::send(when, sock, addr, bytes, secure))
       .map(|_| ())
  }

  pub(crate) fn send(when: When,
                     sock: &mut P::Socket,
                     addr: SocketAddr,
                     bytes: impl Array<Item = u8>,
                     secure: Secure)
                     -> Result<SocketAddr, Error<P>> {
    let len = bytes.get_size();

    nb::block!(match secure {
                 | Secure::IfSupported => sock.send(Addrd(&bytes, addr)),
                 | Secure::No => sock.insecure_send(Addrd(&bytes, addr)),
               }).map_err(|err| when.what(What::SockError(err)))
                 .perform(|()| log::trace!("sent {}b -> {}", len, addr))
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
  /// use toad::core::Core;
  /// use toad::platform::Std;
  /// use toad::req::Req;
  ///
  /// let sock = UdpSocket::bind(("0.0.0.0", 8004)).unwrap();
  /// let mut core = Core::<Std>::new(Default::default(), sock);
  /// let id = core.ping("1.1.1.1", 5683);
  /// // core.poll_ping(id);
  /// ```
  pub fn ping(&mut self,
              host: impl AsRef<str>,
              port: u16)
              -> Result<(toad_msg::Id, SocketAddr), Error<P>> {
    let when = When::None;

    host.as_ref()
        .parse::<IpAddr>()
        .map_err(|_| when.what(What::HostInvalidIpAddress))
        .map(|host| SocketAddr::new(host, port))
        .map(|addr| (addr, self.next_id(addr)))
        .bind(|(addr, id)| {
          let mut req = Req::<P>::get(addr, "");
          req.set_msg_id(id);
          req.set_msg_token(Token(Default::default()));

          let mut msg: platform::Message<P> = req.into();
          msg.opts = Default::default();
          msg.code = toad_msg::Code::new(0, 0);

          Self::send_msg_sock(&mut self.sock, Addrd(msg, addr), Secure::IfSupported).map(|_| {
                                                                                      (id, addr)
                                                                                    })
        })
  }
}

#[cfg(test)]
mod tests {
  use no_std_net::{Ipv4Addr, SocketAddrV4};
  use tinyvec::ArrayVec;
  use toad_msg::TryIntoBytes;

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
                     token: toad_msg::Token(Default::default()),
                     code: toad_msg::Code::new(0, 0),
                     ver: Default::default(),
                     ty: toad_msg::Type::Reset,
                     payload: toad_msg::Payload(Default::default()),
                     opts: Default::default() };

    let _bytes = resp.try_into_bytes::<ArrayVec<[u8; 1152]>>().unwrap();

    // client.fire(Event::RecvDgram(Some((bytes, addr)))).unwrap();
    client.poll_ping(id, addr).unwrap();
  }

  #[test]
  fn client_flow() {
    type Msg = platform::Message<Config>;

    let req = Req::<Config>::get("0.0.0.0:1234".parse().unwrap(), "");
    let token = req.msg.token;
    let resp = Resp::<Config>::for_request(&req).unwrap();
    let bytes = Msg::from(resp).try_into_bytes::<Vec<u8>>().unwrap();

    let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 1234);
    let sock = SockMock::new();
    sock.rx
        .lock()
        .unwrap()
        .push(Addrd(bytes.clone(), addr.into()));
    let mut client = Core::<Config>::new(crate::std::Clock::new(), sock);

    let rep = client.poll_resp(token, addr.into()).unwrap();
    assert_eq!(bytes, Msg::from(rep).try_into_bytes::<Vec<u8>>().unwrap());
  }
}
