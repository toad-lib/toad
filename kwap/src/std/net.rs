use kwap_common::prelude::*;
use std::io;

use crate::net::{Addrd, Socket};

impl Socket for std::net::UdpSocket {
  type Error = io::Error;

  fn send(&self, msg: Addrd<&[u8]>) -> nb::Result<(), Self::Error> {
    self.set_nonblocking(true)
        .bind(|_| std::net::UdpSocket::send_to::<std::net::SocketAddr>(self, msg.data(), addr::no_std::SockAddr(msg.addr()).into()))
        .map(|_| ())
        .map_err(io_to_nb)
  }

  fn recv(&self, buffer: &mut [u8]) -> nb::Result<Addrd<usize>, Self::Error> {
    self.set_nonblocking(true).unwrap();
    self.recv_from(buffer)
        .map(|(n, addr)| Addrd(n, addr::std::SockAddr(addr).into()))
        .map_err(io_to_nb)
  }

  fn bind_raw<A: no_std_net::ToSocketAddrs>(addr: A) -> Result<Self, Self::Error> {
    let addrs = addr.to_socket_addrs()
                    .unwrap()
                    .map(|no_std| addr::no_std::SockAddr(no_std).into())
                    .collect::<Vec<std::net::SocketAddr>>();

    std::net::UdpSocket::bind(addrs.as_slice()).perform(|s| s.set_nonblocking(true).unwrap())
  }

  fn join_multicast(&self, addr: no_std_net::IpAddr) -> Result<(), Self::Error> {
    match addr::std::Ip::from(addr::no_std::Ip(addr)).0 {
      | std::net::IpAddr::V4(addr) => {
        self.join_multicast_v4(&addr, &std::net::Ipv4Addr::UNSPECIFIED)
      },
      | std::net::IpAddr::V6(addr) => self.join_multicast_v6(&addr, 0),
    }
  }
}


mod addr {
  pub(crate) mod std {
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

    use super::no_std;

    #[derive(Copy, Clone, Debug)]
    pub(crate) struct Ipv4(pub(crate) Ipv4Addr);
    #[derive(Copy, Clone, Debug)]
    pub(crate) struct Ipv6(pub(crate) Ipv6Addr);
    #[derive(Copy, Clone, Debug)]
    pub(crate) struct Ip(pub(crate) IpAddr);

    #[derive(Copy, Clone, Debug)]
    pub(crate) struct SockAddrv4(pub(crate) SocketAddrV4);
    #[derive(Copy, Clone, Debug)]
    pub(crate) struct SockAddrv6(pub(crate) SocketAddrV6);
    #[derive(Copy, Clone, Debug)]
    pub(crate) struct SockAddr(pub(crate) SocketAddr);

    impl From<Ipv4> for no_std::Ipv4 {
      fn from(other: Ipv4) -> Self {
        let [a, b, c, d] = other.0.octets();
        no_std::Ipv4(no_std_net::Ipv4Addr::new(a, b, c, d))
      }
    }

    impl From<Ipv6> for no_std::Ipv6 {
      fn from(other: Ipv6) -> Self {
        let [a, b, c, d, e, f, g, h] = other.0.segments();
        no_std::Ipv6(no_std_net::Ipv6Addr::new(a, b, c, d, e, f, g, h))
      }
    }

    impl From<SockAddrv4> for no_std::SockAddrv4 {
      fn from(other: SockAddrv4) -> no_std::SockAddrv4 {
        no_std::SockAddrv4(no_std_net::SocketAddrV4::new(no_std::Ipv4::from(Ipv4(*other.0.ip())).0,
                                                         other.0.port()))
      }
    }

    impl From<SockAddrv6> for no_std::SockAddrv6 {
      fn from(other: SockAddrv6) -> no_std::SockAddrv6 {
        no_std::SockAddrv6(no_std_net::SocketAddrV6::new(no_std::Ipv6::from(Ipv6(*other.0.ip())).0,
                                                         other.0.port(),
                                                         other.0.flowinfo(),
                                                         other.0.scope_id()))
      }
    }

    impl From<Ip> for no_std::Ip {
      fn from(other: Ip) -> Self {
        let inner = match other.0 {
          | IpAddr::V4(ip) => no_std_net::IpAddr::V4(no_std::Ipv4::from(Ipv4(ip)).0),
          | IpAddr::V6(ip) => no_std_net::IpAddr::V6(no_std::Ipv6::from(Ipv6(ip)).0),
        };

        no_std::Ip(inner)
      }
    }

    impl From<SockAddr> for no_std::SockAddr {
      fn from(other: SockAddr) -> no_std::SockAddr {
        let inner = match other.0 {
          | SocketAddr::V4(v4) => {
            no_std_net::SocketAddr::V4(no_std::SockAddrv4::from(SockAddrv4(v4)).0)
          },
          | SocketAddr::V6(v6) => {
            no_std_net::SocketAddr::V6(no_std::SockAddrv6::from(SockAddrv6(v6)).0)
          },
        };
        no_std::SockAddr(inner)
      }
    }

    impl Into<no_std_net::SocketAddr> for SockAddr {
      fn into(self) -> no_std_net::SocketAddr {
        no_std::SockAddr::from(self).0
      }
    }
  }

  pub(crate) mod no_std {
    use no_std_net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

    use super::std as yes_std;

    #[derive(Copy, Clone, Debug)]
    pub(crate) struct Ipv4(pub(crate) Ipv4Addr);
    #[derive(Copy, Clone, Debug)]
    pub(crate) struct Ipv6(pub(crate) Ipv6Addr);
    #[derive(Copy, Clone, Debug)]
    pub(crate) struct Ip(pub(crate) IpAddr);

    #[derive(Copy, Clone, Debug)]
    pub(crate) struct SockAddrv4(pub(crate) SocketAddrV4);
    #[derive(Copy, Clone, Debug)]
    pub(crate) struct SockAddrv6(pub(crate) SocketAddrV6);
    #[derive(Copy, Clone, Debug)]
    pub(crate) struct SockAddr(pub(crate) SocketAddr);

    impl From<Ipv4> for yes_std::Ipv4 {
      fn from(other: Ipv4) -> Self {
        let [a, b, c, d] = other.0.octets();
        yes_std::Ipv4(std::net::Ipv4Addr::new(a, b, c, d))
      }
    }

    impl From<Ipv6> for yes_std::Ipv6 {
      fn from(other: Ipv6) -> Self {
        let [a, b, c, d, e, f, g, h] = other.0.segments();
        yes_std::Ipv6(std::net::Ipv6Addr::new(a, b, c, d, e, f, g, h))
      }
    }

    impl From<SockAddrv4> for yes_std::SockAddrv4 {
      fn from(other: SockAddrv4) -> yes_std::SockAddrv4 {
        yes_std::SockAddrv4(std::net::SocketAddrV4::new(yes_std::Ipv4::from(Ipv4(*other.0.ip())).0,
                                                        other.0.port()))
      }
    }

    impl From<SockAddrv6> for yes_std::SockAddrv6 {
      fn from(other: SockAddrv6) -> yes_std::SockAddrv6 {
        yes_std::SockAddrv6(std::net::SocketAddrV6::new(yes_std::Ipv6::from(Ipv6(*other.0.ip())).0,
                                                        other.0.port(),
                                                        other.0.flowinfo(),
                                                        other.0.scope_id()))
      }
    }
    impl From<Ip> for yes_std::Ip {
      fn from(other: Ip) -> Self {
        let inner = match other.0 {
          | IpAddr::V4(ip) => std::net::IpAddr::V4(yes_std::Ipv4::from(Ipv4(ip)).0),
          | IpAddr::V6(ip) => std::net::IpAddr::V6(yes_std::Ipv6::from(Ipv6(ip)).0),
        };
        yes_std::Ip(inner)
      }
    }

    impl From<SockAddr> for yes_std::SockAddr {
      fn from(other: SockAddr) -> yes_std::SockAddr {
        let inner = match other.0 {
          | SocketAddr::V4(v4) => {
            std::net::SocketAddr::V4(yes_std::SockAddrv4::from(SockAddrv4(v4)).0)
          },
          | SocketAddr::V6(v6) => {
            std::net::SocketAddr::V6(yes_std::SockAddrv6::from(SockAddrv6(v6)).0)
          },
        };
        yes_std::SockAddr(inner)
      }
    }

    impl Into<std::net::SocketAddr> for SockAddr {
      fn into(self) -> std::net::SocketAddr {
        yes_std::SockAddr::from(self).0
      }
    }
  }
}

fn io_to_nb(err: io::Error) -> nb::Error<io::Error> {
  match err.kind() {
    | io::ErrorKind::WouldBlock => nb::Error::WouldBlock,
    | _ => nb::Error::Other(err),
  }
}
