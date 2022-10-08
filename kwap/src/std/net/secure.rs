use core::time::Duration;
use std::collections::HashMap;
use std::io::{self, Write};
use std::net::UdpSocket;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};

use toad_common::prelude::*;
use openssl::ssl::{MidHandshakeSslStream,
                   Ssl,
                   SslAcceptor,
                   SslConnector,
                   SslContext,
                   SslMethod,
                   SslMode};

use super::convert::nb_to_io;
use super::{convert, Addrd, Socket};
use crate::todo::{self, NbResultExt, ResultExt2};

/// Secure socket result
pub type Result<T> = ::core::result::Result<T, Error>;
type Shared<T> = Arc<Mutex<T>>;
type Connections = HashMap<no_std_net::SocketAddr, Shared<conn::SecureUdpConn>>;

#[allow(missing_debug_implementations)]
enum SslRole {
  Server(SslContext),
  Client(SslConnector),
}

#[doc(inline)]
pub use error::*;
mod error {
  use super::*;

  /// I/O errors that sockets secured by DTLS can encounter
  #[derive(Debug)]
  pub enum Error {
    /// There was in issue within openssl - this is more likely
    /// to be a bug in `toad` than a bug in `openssl`.
    Ssl(openssl::ssl::Error),
    /// There was an IO error raised by the underlying socket
    Io(std::io::Error),
    /// A message was received from / outbound to an address
    /// that we haven't established a connection with
    ConnectionNotFound,
    /// The operation would block
    WouldBlock,
    /// TODO probably unnecessary
    WouldBlockMidHandshake(MidHandshakeSslStream<conn::UdpConn>),
  }

  impl From<nb::Error<Error>> for Error {
    fn from(e: nb::Error<Self>) -> Self {
      match e {
        | nb::Error::WouldBlock => Self::WouldBlock,
        | nb::Error::Other(e) => e,
      }
    }
  }

  impl Error {
    pub(super) fn into_nb(self) -> nb::Error<Self> {
      match self {
        | Self::Io(io) if io.kind() == std::io::ErrorKind::WouldBlock => nb::Error::WouldBlock,
        | Self::Ssl(e)
          if e.io_error()
              .map(|io| io.kind() == std::io::ErrorKind::WouldBlock)
              .unwrap_or_default() =>
        {
          nb::Error::WouldBlock
        },
        | Self::WouldBlock => nb::Error::WouldBlock,
        | e => nb::Error::Other(e),
      }
    }
  }

  impl From<openssl::ssl::Error> for Error {
    fn from(e: openssl::ssl::Error) -> Self {
      Self::Ssl(e)
    }
  }

  impl From<openssl::error::ErrorStack> for Error {
    fn from(e: openssl::error::ErrorStack) -> Self {
      Self::Ssl(e.into())
    }
  }

  impl From<openssl::ssl::HandshakeError<conn::UdpConn>> for Error {
    fn from(e: openssl::ssl::HandshakeError<conn::UdpConn>) -> Self {
      match e {
        | openssl::ssl::HandshakeError::SetupFailure(e) => e.into(),
        | openssl::ssl::HandshakeError::Failure(e) => e.into_error().into(),
        | openssl::ssl::HandshakeError::WouldBlock(e) => Self::WouldBlockMidHandshake(e),
      }
    }
  }

  impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
      Self::Io(e)
    }
  }
}

/// # UDP Connections
///
/// Implementations of the io stream traits ([`Read`], [`Write`])
/// for UDP sockets
///
/// You probably don't need to refer to these directly, but you can
/// if you've walked yourself into a deep hole
pub mod conn {
  use super::*;

  pub(in crate::std) type SslStream = openssl::ssl::SslStream<UdpConn>;

  #[derive(Debug, Clone, Copy)]
  enum HandshakeState {
    NotStarted,
    Done,
  }

  /// A raw unsecured UDP stream
  ///
  /// This contains internal state like:
  ///  - A reference to the [`UdpSocket`] that it was created from
  ///  - The remote address it's connected to
  ///  - Whether it's been successfully secured against the remote address
  #[derive(Debug, Clone)]
  pub struct UdpConn {
    sock: Arc<UdpSocket>,
    addr: no_std_net::SocketAddr,
    handshake_state: HandshakeState,
    tx_buf: Vec<u8>,
  }

  impl UdpConn {
    pub(in crate::std) fn new(sock: Arc<UdpSocket>, addr: no_std_net::SocketAddr) -> Self {
      Self { sock,
             addr,
             handshake_state: HandshakeState::NotStarted,
             tx_buf: vec![] }
    }

    pub(in crate::std) fn handshake_done(&mut self) {
      self.handshake_state = HandshakeState::Done;
    }

    pub(in crate::std) fn peek(&self) -> io::Result<no_std_net::SocketAddr> {
      // This is weird -- openssl encourages us to restart
      // the handshake process when a non-blocking socket (usually tcpstream)
      // is not read-ready but it would be a logic error to continually restart
      // until a message has been received.
      //
      // The workaround I've landed on is to block until we receive a message
      // (happy path: this /should/ happen very fast) because we don't really have control
      // over error granularity like differentiating between:
      //  - "this would block because we don't have a message yet"
      //  - "this would block because we got a message from someone else"
      //  - "this would block and it's been a long time since we sent ClientHello"

      let sock = self.sock.as_ref();
      let sock_ref = sock.deref();

      match self.handshake_state {
        // See above comment for why we block here
        | HandshakeState::NotStarted => todo::nb::block!(sock_ref.peek_addr(),
                                                         io_timeout_after = Duration::from_secs(5)),
        // If we're not negotiating DTLS anymore, proceed as a normal
        // non-blocking connection
        | HandshakeState::Done => sock_ref.peek_addr().map_err(nb_to_io),
      }
    }
  }

  impl io::Write for UdpConn {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
      self.tx_buf.extend(buf);
      Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
      let tx = Addrd(self.tx_buf.as_slice(), self.addr);
      Socket::send(self.sock.as_ref(), tx).perform_nb_err(|_| self.tx_buf.clear())
                                          .perform(|_| self.tx_buf.clear())
                                          .map_err(nb_to_io)
    }
  }

  impl io::Read for UdpConn {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
      self.peek()
          .bind(|rx_addr| {
            if rx_addr == self.addr {
              let recv = Socket::recv(self.sock.as_ref(), buf);
              recv.expect_nonblocking("toad::std::net::UdpConn::peek lied!")
            } else {
              // The message in the socket is for someone else,
              // so we should yield
              Err(io::Error::from(io::ErrorKind::WouldBlock))
            }
          })
          .map(|Addrd(n, _)| n)
    }
  }

  pub(crate) enum SecureUdpConn {
    Established(SslStream),
    Establishing(MidHandshakeSslStream<conn::UdpConn>),
  }

  impl SecureUdpConn {
    pub(in crate::std) fn stream(&mut self) -> Option<&mut conn::SslStream> {
      match self {
        | Self::Established(s) => Some(s),
        | Self::Establishing(_) => None,
      }
    }
  }
}

/// A UDP socket secured by DTLS
pub struct SecureUdpSocket {
  sock: Arc<UdpSocket>,
  ssl: SslRole,
  conns: Mutex<Connections>,
}

impl core::fmt::Debug for SecureUdpSocket {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "SecureUdpSocket {{ /* fields hidden */ }}")
  }
}

impl SecureUdpSocket {
  fn new_acceptor(private_key: openssl::pkey::PKey<openssl::pkey::Private>,
                  cert: openssl::x509::X509)
                  -> Result<SslAcceptor> {
    let builder = SslAcceptor::mozilla_intermediate_v5(SslMethod::dtls());

    builder.perform(|_| log::trace!("set private key"))
           .try_perform_mut(|builder| builder.set_private_key(&private_key))
           .perform(|_| log::trace!("set cert"))
           .try_perform_mut(|builder| builder.set_certificate(&cert))
           .map(|builder| builder.build())
           .map_err(Into::into)
           .perform(|_| log::trace!("new acceptor created"))
  }

  fn new_connector() -> Result<SslConnector> {
    let builder = SslConnector::builder(SslMethod::dtls());
    builder.map(|mut builder| {
             let prev_mode = builder.set_mode(SslMode::all());
             let mode = prev_mode & !SslMode::ENABLE_PARTIAL_WRITE;
             builder.set_mode(mode);

             builder.build()
           })
           .map_err(Into::into)
           .perform(|_| log::trace!("new connector created"))
  }

  /// Create a new secure socket for a server
  ///
  /// You need to bring an unsecured [`UdpSocket`] bound to
  /// some address and an [`SslAcceptor`] you've configured manually.
  ///
  /// It is strongly recommended that you use [`SecureUdpSocket::try_new_server`]
  /// instead, as it comes with sensible secure defaults.
  pub fn new_server(ssl: SslAcceptor, sock: UdpSocket) -> Self {
    sock.set_nonblocking(true).unwrap();
    Self { sock: Arc::new(sock),
           ssl: SslRole::Server(ssl.into_context()),
           conns: Default::default() }
  }

  /// Create a new secure socket for a client
  ///
  /// You need to bring an unsecured [`UdpSocket`] bound to
  /// some address and an [`SslConnector`] you've configured manually.
  ///
  /// It is strongly recommended that you use [`SecureUdpSocket::try_new_client`]
  /// instead, as it comes with sensible secure defaults.
  pub fn new_client(ssl: SslConnector, sock: UdpSocket) -> Self {
    sock.set_nonblocking(true).unwrap();
    Self { sock: Arc::new(sock),
           ssl: SslRole::Client(ssl),
           conns: Default::default() }
  }

  /// Create a new secure socket for a server
  ///
  /// You need to bring an unsecured [`UdpSocket`] bound to
  /// some address, a private key, and an X509 certificate.
  ///
  /// This will configure openssl to use many sensible defaults,
  /// such as [Mozilla's recommended server-side TLS configuration](`openssl::ssl::SslAcceptor::mozilla_intermediate_v5`)
  pub fn try_new_server(sock: UdpSocket,
                        private_key: openssl::pkey::PKey<openssl::pkey::Private>,
                        cert: openssl::x509::X509)
                        -> Result<Self> {
    let ssl = Self::new_acceptor(private_key, cert);
    ssl.map(|ssl| Self::new_server(ssl, sock))
  }

  /// Create a new secure socket for a server
  ///
  /// You just need to bring an unsecured [`UdpSocket`].
  ///
  /// This will configure openssl to use many sensible defaults,
  /// such as disabling the openssl `ENABLE_PARTIAL_WRITE` flag.
  pub fn try_new_client(sock: UdpSocket) -> Result<Self> {
    let ssl = Self::new_connector();
    ssl.map(|ssl| Self::new_client(ssl, sock))
  }

  fn connect(ssl: &SslRole,
             sock: Arc<UdpSocket>,
             conns: &mut Connections,
             addr: no_std_net::SocketAddr)
             -> nb::Result<Shared<conn::SecureUdpConn>, Error> {
    let conn = conn::UdpConn::new(sock, addr);
    match ssl {
      | SslRole::Client(connector) => {
        connector.configure()
                 .map_err(Error::from)
                 .map_err(Error::into_nb)
                 .perform_nb_err(|e| log::error!("configure connector failed: {:?}", e))
                 .bind(|conf| {
                   // TODO: can these be enabled?
                   conf.verify_hostname(false)
                       .use_server_name_indication(false)
                       .into_ssl("")
                       .map_err(Error::from)
                       .bind(|ssl| ssl.connect(conn).map_err(Error::from))
                       .perform_mut(|stream| stream.get_mut().handshake_done())
                       .map_err(Error::into_nb)
                 })
                 .perform_nb_err(|e| log::error!("connect failed: {:?}", e))
      },
      | SslRole::Server(_) => {
        log::error!("{}",
                    ["SecureUdpSocket::connect",
                     "called in server mode.",
                     "This is a bug in `toad`",
                     "and should be filed as an issue."].join(" "));
        let not_found = Error::ConnectionNotFound;
        Err(not_found.into())
      },
    }.map(conn::SecureUdpConn::Established)
     .recover(|e| match e {
       | nb::Error::Other(Error::WouldBlockMidHandshake(e)) => {
         Ok(e).map(conn::SecureUdpConn::Establishing)
       },
       | e => Err(e),
     })
     .map(Mutex::new)
     .map(Arc::new)
     .perform(|conn| {
       conns.insert(addr, conn.clone());
     })
  }

  fn accept(ssl: &SslRole,
            sock: Arc<UdpSocket>,
            conns: &mut Connections,
            addr: no_std_net::SocketAddr)
            -> nb::Result<Shared<conn::SecureUdpConn>, Error> {
    let conn = conn::UdpConn::new(sock, addr);

    let client_uh_oh = || {
      let not_found = Error::ConnectionNotFound;
      log::error!("{}",
                  ["SecureUdpSocket::accept",
                   "called in client mode.",
                   "This is a bug in `toad`",
                   "and should be filed as an issue."].join(" "));
      Err(not_found.into())
    };

    let try_accept = |ctx| {
      Ssl::new(ctx).map_err(Error::from)
                   .bind(|ssl| ssl.accept(conn).map_err(Error::from))
                   .map_err(Error::into_nb)
                   .perform_nb_err(|e| log::error!("accept failed: {:?}", e))
    };

    match ssl {
      | SslRole::Server(ctx) => try_accept(ctx),
      | SslRole::Client(_) => client_uh_oh(),
    }.map(conn::SecureUdpConn::Established)
     .recover(|e| match e {
       | nb::Error::Other(Error::WouldBlockMidHandshake(e)) => {
         Ok(e).map(conn::SecureUdpConn::Establishing)
       },
       | e => Err(e),
     })
     .map(Mutex::new)
     .map(Arc::new)
     .perform(|conn| {
       conns.insert(addr, conn.clone());
     })
  }

  pub(crate) fn get_conn_or_connect(&self,
                                    addr: no_std_net::SocketAddr)
                                    -> Result<Shared<conn::SecureUdpConn>> {
    match self.get_conn(addr) {
      | Some(conn) => Ok(conn),
      | None => Self::connect(&self.ssl,
                              self.sock.clone(),
                              &mut self.conns.lock().unwrap(),
                              addr).map_err(Error::from),
    }
  }

  pub(crate) fn get_conn_or_accept(&self,
                                   addr: no_std_net::SocketAddr)
                                   -> Result<Shared<conn::SecureUdpConn>> {
    match self.get_conn(addr) {
      | Some(conn) => Ok(conn),
      | None => Self::accept(&self.ssl,
                             self.sock.clone(),
                             &mut self.conns.lock().unwrap(),
                             addr).map_err(Error::from),
    }
  }

  pub(crate) fn get_conn(&self,
                         addr: no_std_net::SocketAddr)
                         -> Option<Shared<conn::SecureUdpConn>> {
    let conns = self.conns.lock().unwrap();
    conns.get(&addr).cloned()
  }

  // TODO: this may be totally unnecessary
  /// If the handshake succeeds, modify our state and return Error::WouldBlock
  /// so callers just re-enter the flow with this connection having changed to
  /// the Established state
  pub(crate) fn restart_handshake(&self, addr: no_std_net::SocketAddr) -> Error {
    // PANIC: function is private and only ever called
    // when the addr /definitely/ points to an Establishing
    // connection.

    let mid = self.conns.lock().unwrap().remove(&addr).unwrap();

    Arc::try_unwrap(mid).map_err(|_| {
                          // We do not have exclusive access,
                          // meaning that another thread
                          // is currently continuing the handshake.
                          Error::WouldBlock
                        })
                        .map(|mutex| mutex.into_inner().unwrap())
                        .map(|mid| match mid {
                          | conn::SecureUdpConn::Established(_) => unreachable!(),
                          | conn::SecureUdpConn::Establishing(e) => e,
                        })
                        .map(|mid| match mid.handshake().map_err(Error::from) {
                          | Ok(_) => todo!(),
                          | Err(e) => match e {
                            | Error::WouldBlockMidHandshake(e) => {
                              self.conns
                  .lock()
                  .unwrap()
                  .insert(addr,
                          Arc::new(Mutex::new(conn::SecureUdpConn::Establishing(e))));
                              Error::WouldBlock
                            },
                            | e => e,
                          },
                        })
                        .unwrap_err_or(|_| Error::WouldBlock)
  }
}

impl Socket for SecureUdpSocket {
  type Error = Error;

  fn bind_raw<A: no_std_net::ToSocketAddrs>(_: A) -> Result<Self> {
    todo!()
  }

  fn send(&self, msg: Addrd<&[u8]>) -> nb::Result<(), Self::Error> {
    self.get_conn_or_connect(msg.addr())
        .bind(|stream| {
          let mut lock = stream.lock().unwrap();
          match DerefMut::deref_mut(&mut lock) {
            | conn::SecureUdpConn::Established(stream) => {
              stream.ssl_write(msg.data())
                    .map_err(Error::from)
                    .bind(|_| stream.flush().map_err(Error::from))
            },
            | conn::SecureUdpConn::Establishing(_) => Err(self.restart_handshake(msg.addr())),
          }
        })
        .map_err(Error::into_nb)
        .perform_nb_err(|e| log::error!("{:?}", e))
  }

  fn insecure_send(&self, msg: Addrd<&[u8]>) -> nb::Result<(), Self::Error> {
    Socket::send(self.sock.as_ref(), msg).map_err(|e| e.map(Error::from))
                                         .perform_nb_err(|e| log::error!("{:?}", e))
  }

  fn recv(&self, buffer: &mut [u8]) -> nb::Result<Addrd<usize>, Self::Error> {
    self.sock
        .peek_addr()
        .map_err(convert::nb_to_io)
        .map_err(Error::from)
        .bind(|addr| self.get_conn_or_accept(addr).map(|conn| Addrd(conn, addr)))
        .bind(|Addrd(conn, addr)| {
          let mut lock = conn.lock().unwrap();
          match DerefMut::deref_mut(&mut lock) {
            | conn::SecureUdpConn::Established(conn) => conn.ssl_read(buffer)
                                                            .map(|n| Addrd(n, addr))
                                                            .map_err(Error::from),
            | conn::SecureUdpConn::Establishing(_) => Err(self.restart_handshake(addr)),
          }
        })
        .map_err(Error::into_nb)
        .perform_nb_err(|e| log::error!("{:?}", e))
  }

  fn peek(&self, buffer: &mut [u8]) -> nb::Result<Addrd<usize>, Self::Error> {
    self.conns
        .lock()
        .unwrap()
        .iter_mut()
        .find_map(|(addr, conn)| {
          match conn.lock()
                    .unwrap()
                    .stream()
                    .map(|stream| stream.ssl_peek(buffer))
          {
            | None | Some(Ok(0)) => None,
            | Some(Ok(n)) => Some(Ok(Addrd(n, *addr))),
            | Some(Err(err)) => {
              let err = Error::from(err).into_nb();
              Some(Err(err).perform_nb_err(|e| log::error!("{:?}", e)))
            },
          }
        })
        .unwrap_or(Err(nb::Error::WouldBlock))
  }

  /// Multicast and SSL are incompatible, so this always returns `Err(io::ErrorKind::Unsupported)`.
  fn join_multicast(&self, _: no_std_net::IpAddr) -> Result<()> {
    Err(io::Error::from(io::ErrorKind::Unsupported).into()).perform_err(|e| log::error!("{:?}", e))
  }
}
