use kwap_msg::TryFromBytes;

use super::*;
use crate::todo::{Code, CodeKind, Message};

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
        .map(|addrd| addrd.map(|msg| Req::from(msg)))
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
}
