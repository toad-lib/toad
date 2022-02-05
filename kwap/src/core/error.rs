use kwap_msg::TryIntoBytes;

use crate::{config::{Config, self}, socket::Socket};

/// An error encounterable from within Core
#[derive(Debug)]
pub enum Error<Cfg: Config> {
  /// Some socket operation (e.g. connecting to host) failed
  SockError(<<Cfg as Config>::Socket as Socket>::Error),
  /// Serializing a message to bytes failed
  ToBytes(<config::Message<Cfg> as TryIntoBytes>::Error),
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
