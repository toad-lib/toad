//! Future inherent methods on structs in other crates

use kwap_common::Array;
use kwap_msg::*;

/// Future methods on [`kwap_msg::Token`]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token;

impl Token {
  /// Take an arbitrary-length sequence of bytes and turn it into an opaque message token
  ///
  /// Currently uses the BLAKE2 hashing algorithm, but this may change in the future.
  ///
  /// ```
  /// # use kwap::todo::Token;
  ///
  /// let my_token = Token::opaque([0, 1, 2]);
  /// ```
  pub fn opaque(data: &[u8]) -> kwap_msg::Token {
    use blake2::digest::consts::U8;
    use blake2::{Blake2b, Digest};

    let mut digest = Blake2b::<U8>::new();
    digest.update(data);
    kwap_msg::Token(Into::<[u8; 8]>::into(digest.finalize()).into())
  }
}

/// Whether a code is for a request, response, or empty message
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodeKind {
  /// A request code
  Request,
  /// A response code
  Response,
  /// EMPTY
  Empty,
}

/// Future methods on [`kwap_msg::Code`]
pub trait Code {
  /// Get whether this code is for a request, response, or empty message
  ///
  /// ```
  /// use kwap_msg::Code;
  /// # use kwap::todo::{CodeKind, Code as TodoCode};
  ///
  /// let empty: Code = Code::new(0, 0);
  /// assert_eq!(empty.kind(), CodeKind::Empty);
  ///
  /// let req = Code::new(1, 1); // GET
  /// assert_eq!(req.kind(), CodeKind::Request);
  ///
  /// let resp = Code::new(2, 5); // OK CONTENT
  /// assert_eq!(resp.kind(), CodeKind::Response);
  /// ```
  fn kind(&self) -> CodeKind;
}

impl Code for kwap_msg::Code {
  fn kind(&self) -> CodeKind {
    match self.class {
      | 0 => CodeKind::Empty,
      | 1 => CodeKind::Request,
      | _ => CodeKind::Response,
    }
  }
}

/// Future methods on [`kwap_msg::Message`]
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
           code: kwap_msg::Code::new(0, 0),
           payload: Payload(Default::default()),
           opts: Default::default() }
  }
}
