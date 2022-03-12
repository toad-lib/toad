//! Core methods that manage outbound messages.
//!
//! For core methods that manage inbound messages, see [`super::inbound`].

use super::*;
use crate::config::Config;

impl<Cfg: Config> Core<Cfg> {
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

    core::str::from_utf8(&host).map_err(|err|when.what(What::HostInvalidUtf8(err)))
                               .bind(|host| Ipv4Addr::from_str(host).map_err(|_| when.what(What::HostInvalidIpAddress)))
                               .map(|host| SocketAddr::V4(SocketAddrV4::new(host, port)))
                               .try_perform(|addr| {
                                 let t = Addressed(msg.clone(), *addr);
                                 self.retryable(when, t).map(|bam| self.retry_q.push(Some(bam)))
                               })
                               .tupled(|_| msg.try_into_bytes::<ArrayVec<[u8; 1152]>>().map_err(|err| when.what(What::ToBytes(err))))
                               .bind(|(addr, bytes)| Self::send(when, &mut self.sock, addr, bytes))
                               .map(|addr| (token, addr))
  }

  /// Send a raw message down the wire to some remote host.
  ///
  /// You probably want [`send_req`](#method.send_req) or [`ping`](#method.ping) instead.
  pub(super) fn send(when: When, sock: &mut Cfg::Socket,
                     addr: SocketAddr,
                     bytes: impl Array<Item = u8>)
                     -> Result<SocketAddr, Error<Cfg>> {
    // TODO: uncouple from ipv4
    sock.connect(addr)
        .map_err(|err| when.what(What::SockError(err)))
        .try_perform(|_| nb::block!(sock.send(&bytes)).map_err(|err| when.what(What::SockError(err))))
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

    Result::two(
    msg.try_into_bytes::<ArrayVec<[u8; 13]>>()
       .map_err(|err| when.what(What::ToBytes(err)))
     ,  Ipv4Addr::from_str(host.as_ref()).map_err(|_| when.what(What::HostInvalidIpAddress))
    )
       .map(|(bytes, host)| (bytes, SocketAddr::V4(SocketAddrV4::new(host, port))))
       .bind(|(bytes, host)| Self::send(When::SendingMessage(Some(host), id, token), &mut self.sock, host, bytes))
       .map(|addr| (id, addr))
  }
}
