use kwap_msg::to_bytes::MessageToBytesError;
use kwap_msg::{TryIntoBytes, TryFromBytes, MessageParseError};
use no_std_net::SocketAddr;

use crate::config::{self, Config};
use crate::socket::Socket;

use super::event::MatchEvent;

/// An error encounterable from within Core
#[derive(Debug)]
pub struct Error<Cfg: Config> {
  /// The error that occurred. May bring some debug info with it.
  pub inner: ErrorKind<Cfg>,
  /// What were we doing when it happened?
  pub ctx: Context,
  /// A message to assist debugging
  pub msg: Option<&'static str>,
}

/// The context that an error occurred in
#[derive(Debug, Clone, Copy)]
pub enum Context {
  SendingMessage(
      SocketAddr
        , kwap_msg::Id, kwap_msg::Token),
  ParsingMessage(
      SocketAddr
       ),
}

impl<Cfg: Config> Error<Cfg> {
  /// Is this error `FromBytes`?
  pub fn message_parse_error(&self) -> Option<&MessageParseError> {
    match self.inner {
      ErrorKind::FromBytes(ref e) => Some(e),
      _ => None
    }
  }
}

/// A contextless error with some additional debug data attached.
#[derive(Debug)]
pub enum ErrorKind<Cfg: Config> {
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
