use ::std::io;

pub(crate) fn io_to_nb(err: io::Error) -> nb::Error<io::Error> {
  match err.kind() {
    | io::ErrorKind::WouldBlock => nb::Error::WouldBlock,
    | _ => nb::Error::Other(err),
  }
}

pub(crate) fn nb_to_io(err: nb::Error<io::Error>) -> io::Error {
  match err {
    | nb::Error::WouldBlock => io::Error::from(io::ErrorKind::WouldBlock),
    | nb::Error::Other(err) => err,
  }
}

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

  impl From<SockAddr> for no_std_net::SocketAddr {
    fn from(me: SockAddr) -> no_std_net::SocketAddr {
      no_std::SockAddr::from(me).0
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

  impl From<SockAddr> for std::net::SocketAddr {
    fn from(me: SockAddr) -> std::net::SocketAddr {
      yes_std::SockAddr::from(me).0
    }
  }
}
