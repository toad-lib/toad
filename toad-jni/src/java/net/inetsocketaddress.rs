use std::net::ToSocketAddrs;

use super::InetAddress;
use crate::java;

/// `java.net.InetSocketAddress`
pub struct InetSocketAddress(java::lang::Object);

impl InetSocketAddress {
  /// Create a new socket address, using the local wildcard address
  /// as the IP address
  ///
  /// <https://docs.oracle.com/en/java/javase/19/docs/api/java.base/java/net/InetSocketAddress.html#<init>(int)>
  pub fn new_wildcard_address(e: &mut java::Env, port: i32) -> Self {
    static CTOR: java::Constructor<InetSocketAddress, fn(i32)> = java::Constructor::new();
    CTOR.invoke(e, port)
  }

  /// Create a new socket address, resolving the hostname to an IP
  /// address (unless the string is an IP literal)
  ///
  /// <https://docs.oracle.com/en/java/javase/19/docs/api/java.base/java/net/InetSocketAddress.html#%3Cinit%3E(java.lang.String,int)>
  pub fn new_resolved(e: &mut java::Env, host: impl ToString, port: i32) -> Self {
    static CTOR: java::Constructor<InetSocketAddress, fn(String, i32)> = java::Constructor::new();
    CTOR.invoke(e, host.to_string(), port)
  }

  /// Create a new socket address from a known IP and port
  ///
  /// <https://docs.oracle.com/en/java/javase/19/docs/api/java.base/java/net/InetSocketAddress.html#<init>(java.net.InetAddress,int)>
  pub fn new(e: &mut java::Env, addr: InetAddress, port: i32) -> Self {
    static CTOR: java::Constructor<InetSocketAddress, fn(InetAddress, i32)> =
      java::Constructor::new();
    CTOR.invoke(e, addr, port)
  }

  /// Get the IP address
  pub fn address(&self, e: &mut java::Env) -> InetAddress {
    static GET_ADDRESS: java::Method<InetSocketAddress, fn() -> InetAddress> =
      java::Method::new("getAddress");
    GET_ADDRESS.invoke(e, self)
  }

  /// Get the port
  pub fn port(&self, e: &mut java::Env) -> u16 {
    static GET_PORT: java::Method<InetSocketAddress, fn() -> i32> = java::Method::new("getPort");
    GET_PORT.invoke(e, self) as u16
  }

  /// Convert `InetSocketAddress` to `std::net::SocketAddr`
  pub fn to_std(&self, e: &mut java::Env) -> std::net::SocketAddr {
    std::net::SocketAddr::new(self.address(e).to_std(e), self.port(e))
  }

  /// Convert `std::net::SocketAddr` to `InetSocketAddress`
  pub fn from_std(e: &mut java::Env, addr: std::net::SocketAddr) -> Self {
    let ip = InetAddress::from_std(e, addr.ip());
    Self::new(e, ip, addr.port() as i32)
  }
}

java::object_newtype!(InetSocketAddress);
impl java::Class for InetSocketAddress {
  const PATH: &'static str = "java/net/InetSocketAddress";
}
