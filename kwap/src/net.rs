use kwap_common::Array;
use no_std_net::{SocketAddr, IpAddr};
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
pub trait Socket {
  /// The error yielded by socket operations
  type Error: core::fmt::Debug;

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
}

pub fn socket_addr_to_str<'a>(addr: &SocketAddr, buf: &'a mut impl Array<Item = u8>) -> &'a str {
  let bytes = match addr.ip() {
    IpAddr::V4(ip) => &ip.octets().iter().copied().for_each(|b| {
      if b / 200 >= 1 {
        buf.push(b'2');
      } else if b / 100 >= 1 {
        buf.push(b'1');
      }
    }),
    IpAddr::V6(ip) => &ip.octets() as &[u8],
  };

  buf.extend(bytes.iter().copied());

  str::from_utf8(&buf)
}
