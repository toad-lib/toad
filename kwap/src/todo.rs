//! Future inherent methods on structs in other crates

use kwap_common::Array;
use kwap_msg::*;

pub trait Message<PayloadC: Array<Item = u8>, OptC: Array<Item = u8> + 'static, Opts: Array<Item = Opt<OptC>>> {
  /// Create a new message that ACKs this one.
  ///
  /// This needs an [`Id`] to assign to the newly created message.
  ///
  /// ```
  /// // we are a server
  ///
  /// use kwap_msg::{Id, VecMessage as Message};
  /// # use kwap::todo::Message as TodoMessage;
  /// # use std::net::SocketAddr;
  ///
  /// fn server_get_request() -> Option<(SocketAddr, Message)> {
  ///   // Servery sockety things...
  ///   # use std::net::{Ipv4Addr, ToSocketAddrs};
  ///   # use kwap_msg::{Type, Code, Token, Version, Payload};
  ///   # let addr = (Ipv4Addr::new(0, 0, 0, 0), 1234);
  ///   # let addr = addr.to_socket_addrs().unwrap().next().unwrap();
  ///   # let msg = Message { code: Code::new(0, 0),
  ///   #                     id: Id(1),
  ///   #                     ty: Type::Con,
  ///   #                     ver: Version(1),
  ///   #                     token: Token(tinyvec::array_vec!([u8; 8] => 254)),
  ///   #                     opts: vec![],
  ///   #                     payload: Payload(vec![]) };
  ///   # Some((addr, msg))
  /// }
  ///
  /// fn server_send_msg(addr: SocketAddr, msg: Message) -> Result<(), ()> {
  ///   // Message sendy bits...
  ///   # Ok(())
  /// }
  ///
  /// let (addr, req) = server_get_request().unwrap();
  /// let ack_id = Id(req.id.0 + 1);
  /// let ack = req.ack(ack_id);
  ///
  /// server_send_msg(addr, ack).unwrap();
  /// ```
  fn ack(&self, id: Id) -> kwap_msg::Message<PayloadC, OptC, Opts>;
}

impl<PayloadC: Array<Item = u8> + Clone,
      OptC: Array<Item = u8> + Clone + 'static,
      Opts: Array<Item = Opt<OptC>> + Clone> Message<PayloadC, OptC, Opts> for kwap_msg::Message<PayloadC, OptC, Opts>
{
  fn ack(&self, id: Id) -> kwap_msg::Message<PayloadC, OptC, Opts> {
    Self { id,
           token: self.token,
           ver: Default::default(),
           ty: Type::Ack,
           code: Code::new(0, 0),
           payload: Payload(Default::default()),
           opts: Default::default() }
  }
}
