use no_std_net::ToSocketAddrs;

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
  fn send(&mut self, msg: &[u8]) -> nb::Result<(), Self::Error>;

  /// Receive a message farom the `connect`ed host
  fn recv(&mut self, buffer: &mut [u8]) -> nb::Result<usize, Self::Error>;
}
