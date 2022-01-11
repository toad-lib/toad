use std::{io::{self, Error, ErrorKind},
          net::UdpSocket};

use no_std_net::{SocketAddr, ToSocketAddrs};

use crate::{result_ext::ResultExt, socket::Socket};

impl Socket for UdpSocket {
  type Error = io::Error;

  fn connect<A: ToSocketAddrs>(&mut self, addr: A) -> Result<(), Self::Error> {
    let invalid_addr_error = || Error::new(ErrorKind::InvalidInput, "invalid socket addrs".to_string());

    convert_socket_addrs(addr).ok_or_else(invalid_addr_error)
                              .try_perform(|_| self.set_nonblocking(true))
                              .bind(|addrs| UdpSocket::connect(self, &*addrs))
  }

  fn send(&self, msg: &[u8]) -> nb::Result<(), Self::Error> {
    UdpSocket::send(self, msg).map(|_| ()).map_err(io_to_nb)
  }

  fn recv(&self, buffer: &mut [u8]) -> nb::Result<usize, Self::Error> {
    UdpSocket::recv(self, buffer).map_err(io_to_nb)
  }
}

fn io_to_nb(err: io::Error) -> nb::Error<io::Error> {
  match err.kind() {
    | io::ErrorKind::WouldBlock => nb::Error::WouldBlock,
    | _ => nb::Error::Other(err),
  }
}

fn convert_socket_addr_v4(no_std: no_std_net::SocketAddrV4) -> std::net::SocketAddr {
  let [a, b, c, d] = no_std.ip().octets();
  let ip = std::net::Ipv4Addr::new(a, b, c, d);
  std::net::SocketAddr::V4(std::net::SocketAddrV4::new(ip, no_std.port()))
}

fn convert_socket_addr_v6(sock: no_std_net::SocketAddrV6) -> std::net::SocketAddr {
  let [a, b, c, d, e, f, g, h] = sock.ip().segments();
  let ip = std::net::Ipv6Addr::new(a, b, c, d, e, f, g, h);
  std::net::SocketAddr::V6(std::net::SocketAddrV6::new(ip, sock.port(), sock.flowinfo(), sock.scope_id()))
}

fn convert_socket_addrs<A: ToSocketAddrs>(a: A) -> Option<Vec<std::net::SocketAddr>> {
  a.to_socket_addrs().ok().map(|iter| {
                            iter.map(|addr| match addr {
                                  | SocketAddr::V4(sock) => convert_socket_addr_v4(sock),
                                  | SocketAddr::V6(sock) => convert_socket_addr_v6(sock),
                                })
                                .collect()
                          })
}
