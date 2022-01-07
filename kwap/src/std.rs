use no_std_net::{SocketAddr, ToSocketAddrs};

use crate::socket::Socket;

/// implements [`Socket`] for [`std::net::UdpSocket`]
#[derive(Debug, Default)]
pub struct UdpSocket(Option<std::net::UdpSocket>);

impl Socket for UdpSocket {
  type Error = std::io::Error;

  fn connect<A: ToSocketAddrs>(&mut self, addr: A) -> nb::Result<(), Self::Error> {
    todo!()
  }
  fn send(&mut self, msg: &[u8]) -> nb::Result<(), Self::Error> {
    todo!()
  }
  fn recv(&mut self, buffer: &mut [u8]) -> nb::Result<(usize, SocketAddr), Self::Error> {
    todo!()
  }
}
