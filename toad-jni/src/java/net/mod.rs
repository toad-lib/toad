use crate::java;

mod inetsocketaddress;
#[doc(inline)]
pub use inetsocketaddress::InetSocketAddress;

mod inetaddress;
#[doc(inline)]
pub use inetaddress::{Inet4Address, Inet6Address, InetAddress};

mod protocol_family;
#[doc(inline)]
pub use protocol_family::{ProtocolFamily, StandardProtocolFamily};

/// `java.net.SocketAddress`
pub struct SocketAddress(java::lang::Object);
java::object_newtype!(SocketAddress);
impl java::Class for SocketAddress {
  const PATH: &'static str = "java/net/SocketAddress";
}
