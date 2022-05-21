#![allow(clippy::many_single_char_names)]

use std::io;
use std::net::UdpSocket;

use embedded_time::rate::Fraction;
use kwap_common::prelude::*;

use crate::net::{Addrd, Socket};

/// Implement [`embedded_time::Clock`] using [`std::time`] primitives
#[derive(Debug, Clone, Copy)]
pub struct Clock(std::time::Instant);

impl Default for Clock {
  fn default() -> Self {
    Self::new()
  }
}

impl Clock {
  /// Create a new clock
  pub fn new() -> Self {
    Self(std::time::Instant::now())
  }
}

impl embedded_time::Clock for Clock {
  type T = u64;

  // nanoseconds
  const SCALING_FACTOR: Fraction = Fraction::new(1, 1_000_000_000);

  fn try_now(&self) -> Result<embedded_time::Instant<Self>, embedded_time::clock::Error> {
    let now = std::time::Instant::now();
    let elapsed = now.duration_since(self.0);
    Ok(embedded_time::Instant::new(elapsed.as_nanos() as u64))
  }
}

impl Socket for UdpSocket {
  type Error = io::Error;

  fn send(&self, msg: Addrd<&[u8]>) -> nb::Result<(), Self::Error> {
    self.set_nonblocking(true)
        .bind(|_| UdpSocket::send_to(self, msg.data(), std_addr_from_no_std(msg.addr())))
        .map(|_| ())
        .map_err(io_to_nb)
  }

  fn recv(&self, buffer: &mut [u8]) -> nb::Result<Addrd<usize>, Self::Error> {
    UdpSocket::recv_from(self, buffer).map(|(n, addr)| Addrd(n, no_std_addr_from_std(addr)))
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

fn std_addr_v6_from_no_std(no_std: no_std_net::SocketAddrV6) -> std::net::SocketAddr {
  let [a, b, c, d, e, f, g, h] = no_std.ip().segments();
  let ip = std::net::Ipv6Addr::new(a, b, c, d, e, f, g, h);
  std::net::SocketAddr::V6(std::net::SocketAddrV6::new(ip, no_std.port(), no_std.flowinfo(), no_std.scope_id()))
}

fn std_addr_from_no_std(no_std: no_std_net::SocketAddr) -> std::net::SocketAddr {
  match no_std {
    | no_std_net::SocketAddr::V4(sock) => std_addr_v4_from_no_std(sock),
    | no_std_net::SocketAddr::V6(sock) => std_addr_v6_from_no_std(sock),
  }
}

fn no_std_addr_v4_from_std(std: std::net::SocketAddrV4) -> no_std_net::SocketAddr {
  let [a, b, c, d] = std.ip().octets();
  let ip = no_std_net::Ipv4Addr::new(a, b, c, d);
  no_std_net::SocketAddr::V4(no_std_net::SocketAddrV4::new(ip, std.port()))
}

fn no_std_addr_v6_from_std(std: std::net::SocketAddrV6) -> no_std_net::SocketAddr {
  let [a, b, c, d, e, f, g, h] = std.ip().segments();
  let ip = no_std_net::Ipv6Addr::new(a, b, c, d, e, f, g, h);
  no_std_net::SocketAddr::V6(no_std_net::SocketAddrV6::new(ip, std.port(), std.flowinfo(), std.scope_id()))
}

fn no_std_addr_from_std(std: std::net::SocketAddr) -> no_std_net::SocketAddr {
  match std {
    | std::net::SocketAddr::V4(sock) => no_std_addr_v4_from_std(sock),
    | std::net::SocketAddr::V6(sock) => no_std_addr_v6_from_std(sock),
  }
}
