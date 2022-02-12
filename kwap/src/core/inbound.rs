use super::*;

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
  pub fn store_resp(&mut self, ev: &mut Event<Cfg>) -> EventIO {
    let resp = ev.get_mut_resp().unwrap().take().unwrap();
    if let Some(resp) = self.resps.try_push(Some(Addressed(resp.0, resp.1))) {
      // arrayvec is full, remove nones
      self.resps = self.resps.iter_mut().filter_map(|o| o.take()).map(Some).collect();

      // panic if we're still full
      self.resps.push(resp);
    }

    EventIO
  }

  /// Listens for incoming CONfirmable messages and places them on a queue to reply to with ACKs.
  ///
  /// These ACKs are processed whenever the socket is polled (e.g. [`poll_resp`](#method.poll_resp))
  ///
  /// # Panics
  /// panics when msg storage limit reached (e.g. we receive >16 CON requests and have not acked any)
  pub fn ack(&mut self, ev: &mut Event<Cfg>) -> EventIO {
    match ev {
      | Event::RecvResp(Some((ref resp, ref addr))) => {
        if resp.msg_type() == kwap_msg::Type::Con {
          self.fling_q.push(Some(mk_ack::<Cfg>(resp.token(), *addr)));
        }
      },
      | _ => (),
    };

    EventIO
  }

  /// Listens for incoming ACKs and removes any matching CON messages queued for retry.
  ///
  /// # Panics
  /// panics when msg storage limit reached (e.g. 64 pings were sent and we haven't polled for a response of a single one)
  pub fn process_acks(&mut self, ev: &mut Event<Cfg>) -> EventIO {
    let msg = ev.get_mut_msg().unwrap();

    // is the incoming message an ack?
    if let Some((kwap_msg::Message { id, ty: Type::Ack, .. }, addr)) = msg {
      self.unqueue_retry(*id, *addr);
    }

    EventIO
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
    self.sock
        .poll()
        .map(|polled| {
          if let Some(dgram) = polled {
            // allow the state machine to process the incoming message
            self.fire(Event::RecvDgram(Some(dgram))).unwrap();
          }
          ()
        })
        .map_err(Error::SockError)
        .try_perform(|_| self.send_flings())
        .try_perform(|_| self.send_retrys())
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
