use std::io::{self, Error, ErrorKind};
use std::net::{Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6, UdpSocket};
use std::time::Instant;

use embedded_time as emb_time;

use crate::result_ext::ResultExt;
use crate::socket::Socket;

/// Implement [`embedded_time::Clock`] using [`std::time`] primitives
#[derive(Debug, Clone, Copy)]
pub struct Clock(Instant);

impl Default for Clock {
  fn default() -> Self {
    Self::new()
  }
}

impl Clock {
  /// Create a new clock
  pub fn new() -> Self {
    Self(Instant::now())
  }
}

impl emb_time::Clock for Clock {
  type T = u64;

  const SCALING_FACTOR: emb_time::rate::Fraction = emb_time::rate::Fraction::new(1, 1_000_000_000);

  fn try_now(&self) -> Result<emb_time::Instant<Self>, emb_time::clock::Error> {
    let now = Instant::now();
    let elapsed = now.duration_since(self.0);
    Ok(emb_time::Instant::new(elapsed.as_nanos() as u64))
  }
}

impl Socket for UdpSocket {
  type Error = io::Error;

  fn connect<A: no_std_net::ToSocketAddrs>(&mut self, addr: A) -> Result<(), Self::Error> {
    let invalid_addr_error = || Error::new(ErrorKind::InvalidInput, "invalid socket addrs".to_string());

    std_addr_from_no_std(addr)
      .ok_or_else(invalid_addr_error)
      .try_perform(|_| self.set_nonblocking(true))
      .bind(|addrs| UdpSocket::connect(self, &*addrs))
  }

  fn send(&self, msg: &[u8]) -> nb::Result<(), Self::Error> {
    UdpSocket::send(self, msg).map(|_| ()).map_err(io_to_nb)
  }

  fn recv(&self, buffer: &mut [u8]) -> nb::Result<(usize, no_std_net::SocketAddr), Self::Error> {
    UdpSocket::recv_from(self, buffer)
      .map(|(n, addr)| (n, no_std_addr_from_std(addr)))
      .map_err(io_to_nb)
  }
}

fn io_to_nb(err: io::Error) -> nb::Error<io::Error> {
  match err.kind() {
    | io::ErrorKind::WouldBlock => nb::Error::WouldBlock,
    | _ => nb::Error::Other(err),
  }
}

fn std_addr_v4_from_no_std(no_std: no_std_net::SocketAddrV4) -> std::net::SocketAddr {
  let [a, b, c, d] = no_std.ip().octets();
  let ip = std::net::Ipv4Addr::new(a, b, c, d);
  std::net::SocketAddr::V4(std::net::SocketAddrV4::new(ip, no_std.port()))
}

fn std_addr_v6_from_no_std(sock: no_std_net::SocketAddrV6) -> std::net::SocketAddr {
  let [a, b, c, d, e, f, g, h] = sock.ip().segments();
  let ip = Ipv6Addr::new(a, b, c, d, e, f, g, h);
  std::net::SocketAddr::V6(std::net::SocketAddrV6::new(
    ip,
    sock.port(),
    sock.flowinfo(),
    sock.scope_id(),
  ))
}

fn std_addr_from_no_std<A: no_std_net::ToSocketAddrs>(a: A) -> Option<Vec<SocketAddr>> {
  a.to_socket_addrs().ok().map(|iter| {
    iter
      .map(|addr| match addr {
        | no_std_net::SocketAddr::V4(sock) => std_addr_v4_from_no_std(sock),
        | no_std_net::SocketAddr::V6(sock) => std_addr_v6_from_no_std(sock),
      })
      .collect()
  })
}

fn no_std_addr_v4_from_std(no_std: SocketAddrV4) -> no_std_net::SocketAddr {
  let [a, b, c, d] = no_std.ip().octets();
  let ip = no_std_net::Ipv4Addr::new(a, b, c, d);
  no_std_net::SocketAddr::V4(no_std_net::SocketAddrV4::new(ip, no_std.port()))
}

fn no_std_addr_v6_from_std(sock: SocketAddrV6) -> no_std_net::SocketAddr {
  let [a, b, c, d, e, f, g, h] = sock.ip().segments();
  let ip = no_std_net::Ipv6Addr::new(a, b, c, d, e, f, g, h);
  no_std_net::SocketAddr::V6(no_std_net::SocketAddrV6::new(
    ip,
    sock.port(),
    sock.flowinfo(),
    sock.scope_id(),
  ))
}

fn no_std_addr_from_std(addr: SocketAddr) -> no_std_net::SocketAddr {
  match addr {
    | SocketAddr::V4(sock) => no_std_addr_v4_from_std(sock),
    | SocketAddr::V6(sock) => no_std_addr_v6_from_std(sock),
  }
}
