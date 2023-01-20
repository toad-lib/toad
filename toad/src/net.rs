use no_std_net::{SocketAddr, ToSocketAddrs};
use toad_common::*;

/// Data that came from a network socket
#[derive(PartialEq, PartialOrd, Eq, Ord, Hash, Debug, Clone, Copy)]
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

  /// Mutably borrow the contents of the addressed item
  pub fn data_mut(&mut self) -> &mut T {
    &mut self.0
  }

  /// Copy the socket address for the data
  pub fn addr(&self) -> SocketAddr {
    self.1
  }

  /// Turn the entire structure into something else
  pub fn fold<R>(self, f: impl FnOnce(T, SocketAddr) -> R) -> R {
    f(self.0, self.1)
  }
}

impl<T> AsMut<T> for Addrd<T> {
  fn as_mut(&mut self) -> &mut T {
    &mut self.0
  }
}

/// A CoAP network socket
///
/// This mirrors the Udp socket traits in embedded-nal, but allows us to implement them for foreign types (like `std::net::UdpSocket`).
///
/// One notable difference is that `connect`ing is expected to modify the internal state of a [`Socket`],
/// not yield a connected socket type (like [`std::net::UdpSocket::connect`]).
pub trait Socket: Sized {
  /// The error yielded by socket operations
  type Error: core::fmt::Debug;

  /// Buffer type used for receiving and sending datagrams.
  ///
  /// GOTCHA: if the length of the buffer is zero (even if the capacity is greater in the case
  /// of ArrayVec or Vec), no bytes will be read. Make sure you set the length
  /// manually with zero `0u8` filled in each position. (ex. `Vec::resize(_, 1024usize, 0u8)`)
  type Dgram: Array<Item = u8> + AsRef<[u8]> + Clone + core::fmt::Debug + PartialEq;

  /// Get the local address this socket was created from
  fn local_addr(&self) -> SocketAddr;

  /// Create an empty [`Socket::Dgram`] buffer
  ///
  /// (this has a major GOTCHA, see [`Socket::Dgram`].)
  fn empty_dgram() -> Self::Dgram;

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

  /// Send a message to a remote address, bypassing DTLS.
  ///
  /// If the socket type implementing this trait does not participate
  /// in DTLS, then this is just an alias for `send`.
  fn insecure_send(&self, msg: Addrd<&[u8]>) -> nb::Result<(), Self::Error> {
    self.send(msg)
  }

  /// Pull a buffered datagram from the socket, along with the address to the sender.
  ///
  /// This clears the internal reciever queue, meaning that subsequent calls
  /// to `peek` or `recv` will block until a new datagram is received.
  ///
  /// It is expected that (like [`std::net::UdpSocket`]) if the message is larger
  /// than the buffer, those bytes are dropped and not considered an error condition.
  fn recv(&self, buffer: &mut [u8]) -> nb::Result<Addrd<usize>, Self::Error>;

  /// Pull a buffered datagram from the socket, along with the address to the sender.
  ///
  /// This does not clear the internal receiver queue, meaning that subsequent calls
  /// to `peek` or `recv` will yield the same datagram.
  ///
  /// It is expected that (like [`std::net::UdpSocket`]) if the message is larger
  /// than the buffer, those bytes are dropped and not considered an error condition.
  fn peek(&self, buffer: &mut [u8]) -> nb::Result<Addrd<usize>, Self::Error>;

  /// Look at who the sender of the message at the top of the receipt queue
  /// is.
  ///
  /// This should return [`nb::Error::WouldBlock`] if there is no message available.
  ///
  /// # Default Implementation
  /// The default implementation invokes `peek` with a 0-byte capacity array and discards
  /// the `usize` returned by that function.
  ///
  /// This means that it relies on `peek` to _not error_ when the buffer does not
  /// have sufficient capacity for the datagram on the queue.
  fn peek_addr(&self) -> nb::Result<no_std_net::SocketAddr, Self::Error> {
    self.peek(&mut []).map(|Addrd(_, addr)| addr)
  }

  /// Poll the socket for a datagram from the `connect`ed host
  fn poll(&self) -> Result<Option<Addrd<Self::Dgram>>, Self::Error> {
    let mut buf = Self::empty_dgram();
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
