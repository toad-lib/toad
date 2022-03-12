use kwap_msg::MessageParseError;
use kwap_msg::to_bytes::MessageToBytesError;
use no_std_net::SocketAddr;

use crate::config::{Config};
use crate::socket::Socket;

/// The context that an error occurred in
#[derive(Debug, Clone, Copy)]
pub enum When {
  /// We were polling for a message when the error occurred
  Polling,
  /// We were sending a message
  SendingMessage(Option<SocketAddr>, kwap_msg::Id, kwap_msg::Token),
}

impl When {
  /// Construct a specific error from the context the error occurred in
  pub fn what<Cfg: Config>(self, what: What<Cfg>) -> Error<Cfg> {
    Error {when: self, what}
  }
}

/// An error encounterable from within Core
#[derive(Debug)]
pub struct Error<Cfg: Config> {
  /// What happened?
  pub what: What<Cfg>,
  /// What were we doing when it happened?
  pub when: When,
}

impl<Cfg: Config> Error<Cfg> {
  /// Is this error `FromBytes`?
  pub fn message_parse_error(&self) -> Option<&MessageParseError> {
    match self.what {
      | What::FromBytes(ref e) => Some(e),
      | _ => None,
    }
  }
}

/// A contextless error with some additional debug data attached.
#[derive(Debug)]
pub enum What<Cfg: Config> {
  /// Some socket operation (e.g. connecting to host) failed
  SockError(<<Cfg as Config>::Socket as Socket>::Error),
  /// Serializing a message from bytes failed
  FromBytes(MessageParseError),
  /// Serializing a message to bytes failed
  ToBytes(MessageToBytesError),
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
