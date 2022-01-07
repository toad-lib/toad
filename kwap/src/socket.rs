use no_std_net::{SocketAddr, ToSocketAddrs};

/// TODO
pub trait Socket: Default {
  /// TODO
  type Error: core::fmt::Debug;

  /// TODO
  fn connect<A: ToSocketAddrs>(&mut self, addr: A) -> nb::Result<(), Self::Error>;
  /// TODO
  fn send(&mut self, msg: &[u8]) -> nb::Result<(), Self::Error>;
  /// TODO
  fn recv(&mut self, buffer: &mut [u8]) -> nb::Result<(usize, SocketAddr), Self::Error>;
}
