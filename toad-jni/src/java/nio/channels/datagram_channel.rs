use std::io::Write;
use std::sync::RwLock;

use tinyvec::ArrayVec;
use toad::net::Addrd;
use toad_array::Array;

use crate::java::lang::{Integer, Throwable};
use crate::java::net::{InetSocketAddress, ProtocolFamily, SocketAddress, StandardProtocolFamily};
use crate::java::nio::{ByteBuffer, SelectableChannel};
use crate::java::{self, NoUpcast, Nullable, Object, ResultExt, Signature};

/// `java.nio.channels.DatagramChannel`
pub struct DatagramChannel(java::lang::Object);

java::object_newtype!(DatagramChannel);
impl java::Class for DatagramChannel {
  const PATH: &'static str = "java/nio/channels/DatagramChannel";
}

impl DatagramChannel {
  /// Create a [`PeekableDatagramChannel`] from self
  pub fn peekable(self) -> PeekableDatagramChannel {
    self.into()
  }

  /// `java.nio.channels.DatagramChannel.open`
  pub fn open(e: &mut java::Env, proto: StandardProtocolFamily) -> Result<Self, Throwable> {
    static OPEN: java::StaticMethod<DatagramChannel,
                                      fn(ProtocolFamily) -> Result<DatagramChannel, Throwable>> =
      java::StaticMethod::new("open");
    let proto = proto.into_protocol_family(e);
    OPEN.invoke(e, proto)
  }

  /// `java.nio.channels.DatagramChannel.bind`
  pub fn bind(&self, e: &mut java::Env, addr: InetSocketAddress) -> Result<(), Throwable> {
    static BIND: java::Method<DatagramChannel,
                                fn(SocketAddress) -> Result<NoUpcast<DatagramChannel>, Throwable>> =
      java::Method::new("bind");
    let addr = addr.as_socket_address(e);
    BIND.invoke(e, self, addr).map(|_| ())
  }

  /// `java.nio.channels.AbstractSelectableChannel.configureBlocking(bool)`
  pub fn set_blocking(&self, e: &mut java::Env, blocking: bool) {
    static CONFIGURE_BLOCKING: java::Method<DatagramChannel, fn(bool) -> SelectableChannel> =
      java::Method::new("configureBlocking");
    CONFIGURE_BLOCKING.invoke(e, self, blocking);
  }

  /// `java.nio.channels.DatagramChannel.getLocalAddress`
  pub fn get_local_address(&self, e: &mut java::Env) -> InetSocketAddress {
    static GET_LOCAL_ADDRESS: java::Method<DatagramChannel, fn() -> InetSocketAddress> =
      java::Method::new("getLocalAddress");
    GET_LOCAL_ADDRESS.invoke(e, self)
  }

  /// `java.nio.channels.DatagramChannel.send(ByteBuffer, SocketAddress)`
  pub fn send(&self,
              e: &mut java::Env,
              buf: &ByteBuffer,
              dst: &InetSocketAddress)
              -> nb::Result<u32, Throwable> {
    let dst = dst.as_socket_address(e);

    let (buf, dst) = (buf.downcast_ref(e).to_value(e), dst.downcast_ref(e).to_value(e));
    let written = e.call_method(self.0.as_local(),
                                "send",
                                Signature::of::<fn(ByteBuffer, SocketAddress) -> i32>(),
                                &[(&buf).into(), (&dst).into()])
                   .to_throwable(e)
                   .map(|i| i.i().unwrap() as u32);

    if let Ok(0) = written {
      Err(nb::Error::WouldBlock)
    } else {
      written.map_err(nb::Error::Other)
    }
  }

  /// `java.nio.channels.DatagramChannel.send(ByteBuffer, SocketAddress)`
  pub fn recv(&self,
              e: &mut java::Env,
              buf: &mut ByteBuffer)
              -> nb::Result<(u32, InetSocketAddress), Throwable> {
    let buf_ = buf.downcast_ref(e).to_value(e);
    e.call_method(self.0.as_local(),
                  "receive",
                  Signature::of::<fn(ByteBuffer) -> SocketAddress>(),
                  &[(&buf_).into()])
     .to_throwable(e)
     .map_err(nb::Error::Other)
     .and_then(|jv| {
       let read = buf.position(e);
       buf.rewind(e);
       let addr = Nullable::<SocketAddress>::upcast_value(e, jv);
       let addr = addr.into_option(e).ok_or(nb::Error::WouldBlock)?;
       let addr = InetSocketAddress::from_socket_address(e, addr);
       Ok((read, addr))
     })
  }
}

/// Wrapper of [`DatagramChannel`] allowing for
/// peeking at the datagram on the socket (if there is one)
/// without removing it from the socket.
pub struct PeekableDatagramChannel {
  chan: DatagramChannel,
  peeked: RwLock<Option<(InetSocketAddress, usize, ByteBuffer)>>,
}

impl From<DatagramChannel> for PeekableDatagramChannel {
  fn from(chan: DatagramChannel) -> Self {
    Self { chan,
           peeked: RwLock::new(None) }
  }
}

impl From<PeekableDatagramChannel> for DatagramChannel {
  fn from(PeekableDatagramChannel { chan, .. }: PeekableDatagramChannel) -> Self {
    chan
  }
}

impl toad::net::Socket for PeekableDatagramChannel {
  type Error = java::lang::Throwable;
  type Dgram = ArrayVec<[u8; 1152]>;

  fn local_addr(&self) -> no_std_net::SocketAddr {
    let mut e = java::env();
    let e = &mut e;
    self.chan.get_local_address(e).to_no_std(e)
  }

  fn empty_dgram() -> Self::Dgram {
    ArrayVec::new()
  }

  fn bind_raw<A: no_std_net::ToSocketAddrs>(addr: A) -> Result<Self, Self::Error> {
    let mut e = java::env();
    let e = &mut e;

    let addr = addr.to_socket_addrs().unwrap().next().unwrap();
    let addr = InetSocketAddress::from_no_std(e, addr);
    let chan = DatagramChannel::open(e, StandardProtocolFamily::INet)?;
    chan.bind(e, addr)?;
    chan.set_blocking(e, false);
    Ok(chan.peekable())
  }

  fn send(&self, msg: Addrd<&[u8]>) -> nb::Result<(), Self::Error> {
    let mut e = java::env();
    let e = &mut e;

    let Addrd(buf, addr) = msg;

    let buf = ByteBuffer::new(e, buf.iter().copied());
    let dst = InetSocketAddress::from_no_std(e, addr);

    self.chan.send(e, &buf, &dst).map(|_| ())
  }

  fn recv(&self, mut buf: &mut [u8]) -> nb::Result<toad::net::Addrd<usize>, Self::Error> {
    let mut e = java::env();
    let e = &mut e;

    let out = self.peek(buf)?;

    let mut peeked = self.peeked.write().unwrap();
    *peeked = None;

    Ok(out)
  }

  fn peek(&self, buf: &mut [u8]) -> nb::Result<toad::net::Addrd<usize>, Self::Error> {
    let mut e = java::env();
    let e = &mut e;

    let peeked = self.peeked.read().unwrap();

    match peeked.as_ref() {
      | Some((addr, n, javabuf)) => {
        let n = javabuf.write_to(e, 0, n - 1, buf);
        javabuf.rewind(e);
        Ok(Addrd(n, addr.to_no_std(e)))
      },
      | None => {
        drop(peeked);

        let mut peeked = self.peeked.write().unwrap();

        let mut javabuf = ByteBuffer::new(e, buf.iter().copied());
        let (n, addr) = self.chan.recv(e, &mut javabuf)?;
        let addr_no_std = addr.to_no_std(e);
        let n = javabuf.write_to(e, 0, (n as usize) - 1, buf);
        javabuf.rewind(e);

        *peeked = Some((addr, n, javabuf));

        Ok(Addrd(n, addr_no_std))
      },
    }
  }

  fn join_multicast(&self, addr: no_std_net::IpAddr) -> Result<(), Self::Error> {
    todo!()
  }
}

#[cfg(test)]
mod tests {
  use std::net::{SocketAddr, UdpSocket};

  use toad::net::Socket;

  use super::*;
  use crate::java::lang::Byte;

  #[test]
  fn send() {
    struct Addr {
      java: no_std_net::SocketAddr,
      rust: no_std_net::SocketAddr,
    }

    let mut e = crate::test::init();
    let e = &mut e;

    let addr = Addr { java: "127.0.0.1:5683".parse().unwrap(),
                      rust: "127.0.0.1:5684".parse().unwrap() };

    let rust_sock = <UdpSocket as toad::net::Socket>::bind(addr.rust).unwrap();
    let java_sock = PeekableDatagramChannel::bind(addr.java).unwrap();

    let data = r#"{ "foo": "bar", "number": 123 }"#.as_bytes().to_vec();

    java_sock.send(Addrd(data.as_slice(), addr.rust)).unwrap();

    let mut recvd = Vec::new();
    recvd.resize(data.len(), 0);
    let Addrd(_, from) = toad::net::Socket::recv(&rust_sock, &mut recvd).unwrap();

    assert_eq!(from, addr.java);
    assert_eq!(recvd, data);
  }

  #[test]
  fn recv() {
    struct Addr {
      java: no_std_net::SocketAddr,
      rust: no_std_net::SocketAddr,
    }

    let mut e = crate::test::init();
    let e = &mut e;

    let addr = Addr { java: "127.0.0.1:5685".parse().unwrap(),
                      rust: "127.0.0.1:5686".parse().unwrap() };

    let rust_sock = <UdpSocket as toad::net::Socket>::bind(addr.rust).unwrap();
    let java_sock = PeekableDatagramChannel::bind(addr.java).unwrap();

    let data = r#"{ "foo": "bar", "number": 123 }"#.as_bytes().to_vec();

    let mut recvd = Vec::new();
    recvd.resize(data.len(), 0);

    assert!(matches!(java_sock.peek(&mut recvd), Err(nb::Error::WouldBlock)));

    rust_sock.send_to(&data, addr.java.to_string()).unwrap();

    let Addrd(n, from) = java_sock.peek(&mut recvd).unwrap();
    assert_eq!(n, data.len());
    assert_eq!(from, addr.rust);
    assert_eq!(recvd, data);

    let Addrd(n, from) = java_sock.peek(&mut recvd).unwrap();
    assert_eq!(n, data.len());
    assert_eq!(from, addr.rust);
    assert_eq!(recvd, data);

    let Addrd(n, from) = java_sock.recv(&mut recvd).unwrap();
    assert_eq!(n, data.len());
    assert_eq!(from, addr.rust);
    assert_eq!(recvd, data);

    assert!(matches!(java_sock.peek(&mut recvd), Err(nb::Error::WouldBlock)));
  }
}
