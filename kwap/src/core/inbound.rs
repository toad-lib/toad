use kwap_msg::TryFromBytes;

use super::*;
use crate::util::{const_, ignore};

type DgramHandler<R, Cfg> = fn(&mut Core<Cfg>,
                               kwap_msg::Id,
                               kwap_msg::Token,
                               SocketAddr)
                               -> nb::Result<R, <<Cfg as Config>::Socket as Socket>::Error>;

fn mk_ack<Cfg: Config>(tk: kwap_msg::Token, addr: SocketAddr) -> Addressed<config::Message<Cfg>> {
  use kwap_msg::*;
  let msg = config::Message::<Cfg> { id: crate::generate_id(),
                                     token: tk,
                                     ver: Default::default(),
                                     ty: Type::Ack,
                                     code: Code::new(0, 0),
                                     payload: Payload(Default::default()),
                                     opts: Default::default() };

  Addressed(msg, addr)
}

impl<Cfg: Config> Core<Cfg> {
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
      self.fling_q.push(Some(mk_ack::<Cfg>(resp.data().token(), resp.addr())));
    }
  }

  /// Listens for incoming ACKs and removes any matching CON messages queued for retry.
  pub fn process_acks(&mut self, msg: &Addressed<config::Message<Cfg>>) {
    if msg.data().ty == Type::Ack {
      let (id, addr) = (msg.data().id, msg.addr());
      let ix = self.retry_q
                   .iter()
                   .filter_map(|o| o.as_ref())
                   .enumerate()
                   .find(|(_, Retryable(Addressed(con, con_addr), _))| *con_addr == addr && con.id == id)
                   .map(|(ix, _)| ix);

      if let Some(ix) = ix {
        self.retry_q.remove(ix);
      } else {
        // TODO: RESET if we get an ACK for a message we don't expect an ACK for?
      }
    }
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

  fn dgram_recvd(&mut self, when: error::When, dgram: Addressed<crate::socket::Dgram>) -> Result<(), Error<Cfg>> {
    config::Message::<Cfg>::try_from_bytes(dgram.data()).map(|msg| dgram.map(const_(msg)))
                                                        .map_err(|err| when.what(error::What::FromBytes(err)))
                                                        .map(|msg| self.msg_recvd(msg))
  }

  fn msg_recvd(&mut self, msg: Addressed<config::Message<Cfg>>) -> () {
    self.process_acks(&msg);

    if msg.as_ref().map(|m| m.code.class > 1).data() == &true {
      let resp = msg.map(Resp::<Cfg>::from);
      self.store_resp(resp);
    }
  }

  fn poll<R>(&mut self,
             req_id: kwap_msg::Id,
             addr: SocketAddr,
             token: kwap_msg::Token,
             f: DgramHandler<R, Cfg>)
             -> nb::Result<R, Error<Cfg>> {
    let when = When::Polling;

    self.sock
        .poll()
        .map_err(|e| when.what(What::SockError(e)))
        .try_perform(|polled| polled.map(|dgram| self.dgram_recvd(when, dgram)).unwrap_or(Ok(())))
        .try_perform(|_| self.send_flings())
        .try_perform(|_| self.send_retrys())
        .map_err(nb::Error::Other)
        .bind(|_| f(self, req_id, token, addr).map_err(|e| e.map(|e| when.what(What::SockError(e)))))
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
          | o @ Some(_) if resp_matches(o) => Option::take(o).map(|Addressed(resp, _)| resp),
          | _ => None,
        })
        .ok_or(nb::Error::WouldBlock)
  }

  fn check_ping(&mut self,
                req_id: kwap_msg::Id,
                _: kwap_msg::Token,
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
}
