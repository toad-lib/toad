use kwap_common::prelude::*;
use no_std_net::{SocketAddr, ToSocketAddrs};
use tinyvec::ArrayVec;

/// Data that came from a network socket
#[derive(Debug, Clone, Copy)]
pub struct Addrd<T>(pub T, pub SocketAddr);

impl<T> Addrd<T> {
  /// Borrow the contents of this Addressed
  pub fn as_ref(&self) -> Addrd<&T> {
    Addrd(self.data(), self.addr())
  }

  /// Discard the socket and get the data in this Addressed
  pub fn unwrap(self) -> T {
    self.0
  }

  /// Map the data contained in this Addressed
  pub fn map<R>(self, f: impl FnOnce(T) -> R) -> Addrd<R> {
    Addrd(f(self.0), self.1)
  }

  /// Map the data contained in this Addressed (with a copy of the address)
  pub fn map_with_addr<R>(self, f: impl FnOnce(T, SocketAddr) -> R) -> Addrd<R> {
    Addrd(f(self.0, self.1), self.1)
  }

  /// Borrow the contents of the addressed item
  pub fn data(&self) -> &T {
    &self.0
  }

  /// Copy the socket address for the data
  pub fn addr(&self) -> SocketAddr {
    self.1
  }
}

impl<T> AsMut<T> for Addrd<T> {
  fn as_mut(&mut self) -> &mut T {
    &mut self.0
  }
}

/// A packet recieved over a UDP socket.
///
/// Currently the capacity is hard-coded at 1152 bytes,
/// but this will eventually be configurable at compile-time.
pub type Dgram = ArrayVec<[u8; 1152]>;

/// A CoAP network socket
///
/// This mirrors the Udp socket traits in embedded-nal, but allows us to implement them for foreign types (like `std::net::UdpSocket`).
///
/// One notable difference is that `connect`ing is expected to modify the internal state of a [`Socket`],
/// not yield a connected socket type (like [`std::net::UdpSocket::connect`]).
pub trait Socket: Sized {
  /// The error yielded by socket operations
  type Error: core::fmt::Debug;

  /// Bind the socket to an address, without doing any spooky magic things like switching to non-blocking mode
  /// or auto-detecting and joining multicast groups.
  ///
  /// Implementors of `bind_raw` should:
  ///  - yield a socket in a non-blocking state
  ///  - bind to the first address if `addr` yields multiple addresses
  fn bind_raw<A: ToSocketAddrs>(addr: A) -> Result<Self, Self::Error>;

  /// Binds the socket to a local address.
  ///
  /// The behavior of `addr` yielding multiple addresses is implementation-specific,
  /// but will most likely bind to the first address that is available.
  ///
  /// This function will automatically invoke [`Socket::join_multicast`] if the address
  /// is a multicast address, and should yield a non-blocking socket.
  fn bind<A: ToSocketAddrs>(addr: A) -> Result<Self, Self::Error> {
    let addr = addr.to_socket_addrs().unwrap().next().unwrap();

    Self::bind_raw(addr).try_perform(|sock| match addr.ip() {
                          | ip if ip.is_multicast() => sock.join_multicast(ip),
                          | _ => Ok(()),
                        })
  }

  /// Send a message to a remote address
  fn send(&self, msg: Addrd<&[u8]>) -> nb::Result<(), Self::Error>;

  /// Pull a buffered datagram from the socket, along with the address to the sender.
  fn recv(&self, buffer: &mut [u8]) -> nb::Result<Addrd<usize>, Self::Error>;

  /// Poll the socket for a datagram from the `connect`ed host
  fn poll(&self) -> Result<Option<Addrd<Dgram>>, Self::Error> {
    let mut buf = [0u8; 1152];
    let recvd = self.recv(&mut buf);

    match recvd {
      | Ok(Addrd(n, addr)) => Ok(Some(Addrd(buf.into_iter().take(n).collect(), addr))),
      | Err(nb::Error::WouldBlock) => Ok(None),
      | Err(nb::Error::Other(e)) => Err(e),
    }
  }

  /// Join a multicast group
  fn join_multicast(&self, addr: no_std_net::IpAddr) -> Result<(), Self::Error>;
}
