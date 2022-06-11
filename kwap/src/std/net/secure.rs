use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::net::UdpSocket;
use std::ops::Deref;
use std::sync::{Arc, Mutex};

use embedded_time::duration::Seconds;
use kwap_common::prelude::*;
use openssl::ssl::{ConnectConfiguration, SslAcceptor, SslConnector, SslMethod, SslStream};

use super::convert::nb_to_io;
use super::{convert, Addrd, Socket};
use crate::todo::ResultExt2;

/// TODO
#[derive(Debug)]
pub enum SecureSocketError {
  /// TODO
  Ssl(openssl::ssl::Error),
  /// TODO
  Io(std::io::Error),
  /// A message was received from / outbound to an address
  /// that we haven't established a connection with
  ConnectionNotFound,
}

impl From<nb::Error<SecureSocketError>> for SecureSocketError {
  fn from(e: nb::Error<Self>) -> Self {
    match e {
      | nb::Error::WouldBlock => Self::Io(convert::nb_to_io(nb::Error::WouldBlock)),
      | nb::Error::Other(e) => e,
    }
  }
}

impl SecureSocketError {
  fn into_nb(self) -> nb::Error<Self> {
    match self {
      | SecureSocketError::Io(io) if io.kind() == std::io::ErrorKind::WouldBlock => {
        nb::Error::WouldBlock
      },
      | SecureSocketError::Ssl(e)
        if e.io_error()
            .map(|io| io.kind() == std::io::ErrorKind::WouldBlock)
            .unwrap_or_default() =>
      {
        nb::Error::WouldBlock
      },
      | e => nb::Error::Other(e),
    }
  }
}

impl From<openssl::ssl::Error> for SecureSocketError {
  fn from(e: openssl::ssl::Error) -> Self {
    Self::Ssl(e)
  }
}

impl From<openssl::error::ErrorStack> for SecureSocketError {
  fn from(e: openssl::error::ErrorStack) -> Self {
    Self::Ssl(e.into())
  }
}

impl<S> From<openssl::ssl::HandshakeError<S>> for SecureSocketError {
  fn from(e: openssl::ssl::HandshakeError<S>) -> Self {
    match e {
      | openssl::ssl::HandshakeError::SetupFailure(e) => e.into(),
      | openssl::ssl::HandshakeError::Failure(e) => e.into_error().into(),
      | openssl::ssl::HandshakeError::WouldBlock(_) => {
        convert::nb_to_io(nb::Error::WouldBlock).into()
      },
    }
  }
}

impl From<std::io::Error> for SecureSocketError {
  fn from(e: std::io::Error) -> Self {
    Self::Io(e)
  }
}

/// TODO
#[derive(Debug, Clone)]
pub struct UdpConn {
  sock: Arc<UdpSocket>,
  addr: no_std_net::SocketAddr,
  tx_buf: Vec<u8>,
}

impl UdpConn {
  fn new(sock: Arc<UdpSocket>, addr: no_std_net::SocketAddr) -> Self {
    Self { sock,
           addr,
           tx_buf: vec![] }
  }
}

impl io::Write for UdpConn {
  fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
    self.tx_buf = [&self.tx_buf, buf].concat();
    Ok(buf.len())
  }

  fn flush(&mut self) -> io::Result<()> {
    Socket::send(self.sock.as_ref(), Addrd(&self.tx_buf, self.addr)).map_err(nb_to_io)
  }
}

impl io::Read for UdpConn {
  fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
    let sock = self.sock.as_ref();
    let sock_ref = sock.deref();

    sock_ref.peek_addr()
            .validate(|rx_addr| {
              rx_addr.should_eq(&self.addr)
                     .else_err(|_| nb::Error::WouldBlock)
            })
            .bind(|_| Socket::recv(sock_ref, buf))
            .map_err(nb_to_io)
            .map(|Addrd(n, _)| n)
  }
}

type Shared<T> = Arc<Mutex<T>>;
type SecureUdpConn = SslStream<UdpConn>;
type Connections = HashMap<no_std_net::SocketAddr, Shared<SecureUdpConn>>;

#[allow(missing_debug_implementations)]
enum Ssl {
  Server(SslAcceptor),
  Client(SslConnector),
}

/// TODO
#[allow(missing_debug_implementations)]
pub struct SecureUdpSocket {
  sock: Arc<UdpSocket>,
  ssl: Ssl,
  conns: Mutex<Connections>,
}

impl SecureUdpSocket {
  fn new_acceptor(private_key: openssl::pkey::PKey<openssl::pkey::Private>,
                  cert: openssl::x509::X509)
                  -> Result<SslAcceptor, SecureSocketError> {
    let builder = SslAcceptor::mozilla_intermediate_v5(SslMethod::dtls());
    builder.bind(|mut builder| builder.set_private_key(&private_key).map(|_| builder))
           .bind(|mut builder| builder.set_certificate(&cert).map(|_| builder))
           .map(|builder| builder.build())
           .map_err(Into::into)
  }

  fn new_connector() -> Result<SslConnector, SecureSocketError> {
    let builder = SslConnector::builder(SslMethod::dtls());
    builder.map(|builder| builder.build()).map_err(Into::into)
  }

  /// TODO
  pub fn try_new_server(sock: UdpSocket,
                        private_key: openssl::pkey::PKey<openssl::pkey::Private>,
                        cert: openssl::x509::X509)
                        -> Result<Self, SecureSocketError> {
    let acceptor = Self::new_acceptor(private_key, cert);
    acceptor.map(|ssl| Self { sock: Arc::new(sock),
                              ssl: Ssl::Server(ssl),
                              conns: Default::default() })
  }

  /// TODO
  pub fn try_new_client(sock: UdpSocket) -> Result<Self, SecureSocketError> {
    let ssl = Self::new_connector();
    ssl.map(|ssl| Self { sock: Arc::new(sock),
                         ssl: Ssl::Client(ssl),
                         conns: Default::default() })
  }

  fn new_stream(ssl: &Ssl,
                sock: Arc<UdpSocket>,
                streams: &mut Connections,
                addr: no_std_net::SocketAddr)
                -> nb::Result<Shared<SecureUdpConn>, SecureSocketError> {
    let stream = UdpConn::new(sock, addr);
    match ssl {
      | Ssl::Client(conn) => {
        conn.configure()
            .map_err(SecureSocketError::from)
            .and_then(|conf| {
              // TODO: can these be enabled?
              conf.verify_hostname(false)
                  .use_server_name_indication(false)
                  .connect("", stream)
                  .map_err(Into::into)
            })
            .map_err(SecureSocketError::into_nb)
      },
      | Ssl::Server(_) => Err(SecureSocketError::ConnectionNotFound.into()),
    }.map(Mutex::new)
     .map(Arc::new)
     .perform(|shared| {
       streams.insert(addr, shared.clone());
     })
  }

  pub(crate) fn get_or_establish_conn(&self,
                                      addr: no_std_net::SocketAddr)
                                      -> Result<Shared<SecureUdpConn>, SecureSocketError> {
    match self.get_conn(addr) {
      | Some(conn) => Ok(conn),
      | None => nb::block!(Self::new_stream(&self.ssl,
                                            self.sock.clone(),
                                            &mut self.conns.lock().unwrap(),
                                            addr)).map_err(Into::into),
    }
  }

  pub(crate) fn get_conn(&self, addr: no_std_net::SocketAddr) -> Option<Shared<SecureUdpConn>> {
    let mut conns = self.conns.lock().unwrap();
    conns.get(&addr).cloned()
  }
}

impl Socket for SecureUdpSocket {
  type Error = SecureSocketError;

  fn bind_raw<A: no_std_net::ToSocketAddrs>(_: A) -> Result<Self, Self::Error> {
    todo!()
  }

  fn send(&self, msg: Addrd<&[u8]>) -> nb::Result<(), Self::Error> {
    self.get_or_establish_conn(msg.addr())
        .and_then(|stream| {
          let mut stream = stream.lock().unwrap();
          stream.write(msg.data())
                .and_then(|_| stream.flush())
                .map_err(SecureSocketError::from)
        })
        .map_err(SecureSocketError::into_nb)
  }

  fn recv(&self, buffer: &mut [u8]) -> nb::Result<Addrd<usize>, Self::Error> {
    self.sock
        .peek_addr()
        .map_err(convert::nb_to_io)
        .map_err(SecureSocketError::from)
        .bind(|addr| {
          self.get_conn(addr)
              .ok_or(SecureSocketError::ConnectionNotFound)
              .map(|conn| Addrd(conn, addr))
        })
        .bind(|Addrd(conn, addr)| {
          conn.lock()
              .unwrap()
              .read(buffer)
              .map(|n| Addrd(n, addr))
              .map_err(SecureSocketError::from)
        })
        .map_err(SecureSocketError::into_nb)
  }

  fn peek(&self, buffer: &mut [u8]) -> nb::Result<Addrd<usize>, Self::Error> {
    self.conns
        .lock()
        .unwrap()
        .iter_mut()
        .find_map(|(addr, conn)| match conn.lock().unwrap().ssl_peek(buffer) {
          | Ok(0) => None,
          | Ok(n) => Some(Ok(Addrd(n, *addr))),
          | Err(err) => Some(Err(SecureSocketError::from(err).into_nb())),
        })
        .unwrap_or(Err(nb::Error::WouldBlock))
  }

  /// Multicast and SSL are incompatible, so this always returns `Err(io::ErrorKind::Unsupported)`.
  fn join_multicast(&self, _: no_std_net::IpAddr) -> Result<(), Self::Error> {
    Err(io::Error::from(io::ErrorKind::Unsupported).into())
  }
}
