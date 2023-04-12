use crate::java;

/// `java.net.DatagramSocket`
pub struct DatagramSocket(java::lang::Object);

impl DatagramSocket {
}

java::object_newtype!(DatagramSocket);
impl java::Class for DatagramSocket {
    const PATH: &'static str = "java/net/DatagramSocket";
}

impl toad::net::Socket for DatagramSocket {
    type Error;
    type Dgram = ArrayVec<[u8; 1152]>;

    fn local_addr(&self) -> SocketAddr {
        todo!()
    }

    fn empty_dgram() -> Self::Dgram {
        todo!()
    }

    fn bind_raw<A: ToSocketAddrs>(addr: A) -> Result<Self, Self::Error> {
        todo!()
    }

    fn send(&self, msg: toad::net::Addrd<&[u8]>) -> nb::Result<(), Self::Error> {
        todo!()
    }

    fn recv(&self, buffer: &mut [u8]) -> nb::Result<toad::net::Addrd<usize>, Self::Error> {
        todo!()
    }

    fn peek(&self, buffer: &mut [u8]) -> nb::Result<toad::net::Addrd<usize>, Self::Error> {
        todo!()
    }

    fn join_multicast(&self, addr: no_std_net::IpAddr) -> Result<(), Self::Error> {
        todo!()
    }
}
