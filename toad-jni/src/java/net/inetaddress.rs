use java::Object;

use crate::java;

/// `java.net.Inet4Address`
pub struct Inet4Address(java::lang::Object);
java::object_newtype!(Inet4Address);
impl java::Class for Inet4Address {
  const PATH: &'static str = "java/net/Inet4Address";
}

/// `java.net.Inet6Address`
pub struct Inet6Address(java::lang::Object);
java::object_newtype!(Inet6Address);
impl java::Class for Inet6Address {
  const PATH: &'static str = "java/net/Inet6Address";
}

/// `java.net.InetAddress`
#[allow(missing_docs)]
pub enum InetAddress {
  V4(Inet4Address),
  V6(Inet6Address),
}

static INETADDRESS_GET_BY_ADDRESS: java::StaticMethod<InetAddress, fn(Vec<i8>) -> InetAddress> =
  java::StaticMethod::new("getByAddress");

macro_rules! to_net_impl {
  (use $crate_:path; $self_:expr, $e:expr) => {{
    use $crate_::IpAddr;
    let bytes = $self_.get_address($e);
    match $self_ {
      | Self::V4(_) => {
        let bytes: [u8; 4] = bytes.as_slice().try_into().unwrap();
        IpAddr::from(bytes)
      },
      | Self::V6(_) => {
        let bytes: [u8; 16] = bytes.as_slice().try_into().unwrap();
        IpAddr::from(bytes)
      },
    }
  }};
}

macro_rules! from_net_impl {
  (use $crate_:path; $e:expr, $addr:expr) => {{
    use $crate_::IpAddr;
    match $addr {
      | IpAddr::V4(ip) => Self::new_ipv4($e, ip.octets()),
      | IpAddr::V6(ip) => Self::new_ipv6($e, ip.octets()),
    }
  }};
}

impl InetAddress {
  /// [`InetAddress getByAddress(byte[])`](https://docs.oracle.com/en/java/javase/19/docs/api/java.base/java/net/InetAddress.html#getByAddress(byte%5B%5D))
  pub fn new_ipv4(e: &mut java::Env, addr: [u8; 4]) -> Self {
    INETADDRESS_GET_BY_ADDRESS.invoke(e,
                                      addr.iter()
                                          .copied()
                                          .map(|u| i8::from_be_bytes(u.to_be_bytes()))
                                          .collect())
  }

  /// [`InetAddress getByAddress(byte[])`](https://docs.oracle.com/en/java/javase/19/docs/api/java.base/java/net/InetAddress.html#getByAddress(byte%5B%5D))
  pub fn new_ipv6(e: &mut java::Env, addr: [u8; 16]) -> Self {
    INETADDRESS_GET_BY_ADDRESS.invoke(e,
                                      addr.iter()
                                          .copied()
                                          .map(|u| i8::from_be_bytes(u.to_be_bytes()))
                                          .collect())
  }

  fn get_address(&self, e: &mut java::Env) -> Vec<u8> {
    static GET_ADDRESS: java::Method<InetAddress, fn() -> Vec<i8>> =
      java::Method::new("getAddress");

    let bytes = GET_ADDRESS.invoke(e, self);
    bytes.iter()
         .copied()
         .map(|i| u8::from_be_bytes(i.to_be_bytes()))
         .collect::<Vec<u8>>()
  }

  /// Convert `InetAddress` to `std::net::IpAddr`
  pub fn to_std(&self, e: &mut java::Env) -> std::net::IpAddr {
    to_net_impl!(use std::net;, self, e)
  }

  /// Convert `InetAddress` to `no_std_net::IpAddr`
  pub fn to_no_std(&self, e: &mut java::Env) -> no_std_net::IpAddr {
    to_net_impl!(use no_std_net;, self, e)
  }

  /// Convert `std::net::IpAddr` to `InetAddress`
  pub fn from_std(e: &mut java::Env, addr: std::net::IpAddr) -> Self {
    from_net_impl!(use std::net;, e, addr)
  }

  /// Convert `no_std_net::IpAddr` to `InetAddress`
  pub fn from_no_std(e: &mut java::Env, addr: no_std_net::IpAddr) -> Self {
    from_net_impl!(use no_std_net;, e, addr)
  }
}

impl java::Object for InetAddress {
  fn upcast(e: &mut java::Env, jobj: java::lang::Object) -> Self {
    if jobj.is_instance_of::<Inet4Address>(e) {
      Self::V4(jobj.upcast_to::<Inet4Address>(e))
    } else {
      Self::V6(jobj.upcast_to::<Inet6Address>(e))
    }
  }

  fn downcast(self, e: &mut java::Env) -> java::lang::Object {
    match self {
      | Self::V4(o) => o.downcast(e),
      | Self::V6(o) => o.downcast(e),
    }
  }

  fn downcast_ref(&self, e: &mut java::Env) -> java::lang::Object {
    match self {
      | Self::V4(o) => o.downcast_ref(e),
      | Self::V6(o) => o.downcast_ref(e),
    }
  }
}

impl java::Class for InetAddress {
  const PATH: &'static str = "java/net/InetAddress";
}
