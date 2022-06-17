use std::io;
use std::net::UdpSocket;

use kwap_common::prelude::*;

use crate::net::{Addrd, Socket};

mod convert;

/// TODO
pub mod secure;
pub use secure::*;

impl Socket for UdpSocket {
  type Error = io::Error;

  fn send(&self, msg: Addrd<&[u8]>) -> nb::Result<(), Self::Error> {
    self.set_nonblocking(true)
        .bind(|_| {
          UdpSocket::send_to::<std::net::SocketAddr>(self,
                                                     msg.data(),
                                                     convert::no_std::SockAddr(msg.addr()).into())
        })
        .map(|_| ())
        .map_err(convert::io_to_nb)
  }

  fn recv(&self, buffer: &mut [u8]) -> nb::Result<Addrd<usize>, Self::Error> {
    self.set_nonblocking(true).unwrap();
    self.recv_from(buffer)
        .map(|(n, addr)| Addrd(n, convert::std::SockAddr(addr).into()))
        .map_err(convert::io_to_nb)
  }

  fn bind_raw<A: no_std_net::ToSocketAddrs>(addr: A) -> Result<Self, Self::Error> {
    let addrs = addr.to_socket_addrs()
                    .unwrap()
                    .map(|no_std| convert::no_std::SockAddr(no_std).into())
                    .collect::<Vec<std::net::SocketAddr>>();

    UdpSocket::bind(addrs.as_slice()).perform(|s| s.set_nonblocking(true).unwrap())
  }

  fn join_multicast(&self, addr: no_std_net::IpAddr) -> Result<(), Self::Error> {
    match convert::std::Ip::from(convert::no_std::Ip(addr)).0 {
      | std::net::IpAddr::V4(addr) => {
        self.join_multicast_v4(&addr, &std::net::Ipv4Addr::UNSPECIFIED)
      },
      | std::net::IpAddr::V6(addr) => self.join_multicast_v6(&addr, 0),
    }
  }

  fn peek(&self, buffer: &mut [u8]) -> nb::Result<Addrd<usize>, Self::Error> {
    std::net::UdpSocket::peek_from(self, buffer).map(|(n, addr)| {
                                                   Addrd(n,
            convert::no_std::SockAddr::from(convert::std::SockAddr(addr)).0)
                                                 })
                                                 .map_err(convert::io_to_nb)
  }
}
