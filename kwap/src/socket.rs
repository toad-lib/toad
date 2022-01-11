use no_std_net::ToSocketAddrs;
use tinyvec::ArrayVec;

/// A CoAP network socket
///
/// This mirrors the Udp socket traits in embedded-nal, but allows us to implement them for foreign types (like `std::net::UdpSocket`).
///
/// One notable difference is that `connect`ing is expected to modify the internal state of a [`Socket`],
/// not yield a connected socket type (like [`std::net::UdpSocket::connect`]).
pub trait Socket {
  /// The error yielded by socket operations
  type Error: core::fmt::Debug;

  /// Connect as a client to some remote host
  fn connect<A: ToSocketAddrs>(&mut self, addr: A) -> nb::Result<(), Self::Error>;

  /// Send a message to the `connect`ed host
  fn send(&self, msg: &[u8]) -> nb::Result<(), Self::Error>;

  /// Receive a message farom the `connect`ed host
  fn recv(&self, buffer: &mut [u8]) -> nb::Result<usize, Self::Error>;

  /// Poll the socket for a datagram
  ///
  fn poll(&self) -> Result<Option<ArrayVec<[u8; 1152]>>, Self::Error> {
    let mut buf = [0u8; 1152];
    let recvd = self.recv(&mut buf);

    match recvd {
      | Ok(n) => Ok(Some(buf[0..n].iter().copied().collect())),
      | Err(nb::Error::WouldBlock) => Ok(None),
      | Err(nb::Error::Other(e)) => Err(e),
    }
  }
}
