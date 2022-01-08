use std::{io::{self, Error, ErrorKind},
          net::UdpSocket};

use no_std_net::{SocketAddr, ToSocketAddrs};

use crate::socket::Socket;

impl Socket for UdpSocket {
  type Error = io::Error;

  fn connect<A: ToSocketAddrs>(&mut self, addr: A) -> nb::Result<(), Self::Error> {
    convert_socket_addrs(addr).ok_or_else(|| {
                                nb::Error::Other(Error::new(ErrorKind::InvalidInput,
                                                            "invalid socket addrs".to_string()))
                              })
                              .and_then(|addr| UdpSocket::connect(self, &*addr).map_err(|e| nb::Error::Other(e)))
  }

  fn send(&mut self, msg: &[u8]) -> nb::Result<(), Self::Error> {
    UdpSocket::send(self, msg).map(|_| ()).map_err(nb::Error::Other)
  }

  fn recv(&mut self, buffer: &mut [u8]) -> nb::Result<usize, Self::Error> {
    UdpSocket::recv(self, buffer).map_err(nb::Error::Other)
  }
}

fn convert_socket_addrs<A: ToSocketAddrs>(a: A) -> Option<Vec<std::net::SocketAddr>> {
  a.to_socket_addrs().ok().map(|iter| {
                            iter.map(|addr| match addr {
                                  | SocketAddr::V4(sock) => {
                                    let ip = sock.ip().octets();
                                    let ip = std::net::Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]);
                                    std::net::SocketAddr::V4(std::net::SocketAddrV4::new(ip, sock.port()))
                                  },
                                  | SocketAddr::V6(sock) => {
                                    let ip = sock.ip().segments();
                                    let ip =
                                      std::net::Ipv6Addr::new(ip[0], ip[1], ip[2], ip[3], ip[4], ip[5], ip[6], ip[7]);
                                    std::net::SocketAddr::V6(std::net::SocketAddrV6::new(ip,
                                                                                         sock.port(),
                                                                                         sock.flowinfo(),
                                                                                         sock.scope_id()))
                                  },
                                })
                                .collect()
                          })
}
