#![allow(dead_code)]

use ::core::cell::Cell;
use ::core::ops::Deref;
use ::core::time::Duration;
use ::std::sync::Mutex;
use ::std::thread;
use embedded_time::rate::Fraction;
use embedded_time::Instant;
use net::*;
use no_std_net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std_alloc::sync::Arc;
use toad_msg::{TryFromBytes, TryIntoBytes};

use super::*;

#[macro_export]
macro_rules! msg {
  (CON GET x.x.x.x:$port:literal) => { $crate::test::msg!(CON {1 . 1} x.x.x.x:$port) };
  (CON PUT x.x.x.x:$port:literal) => { $crate::test::msg!(CON {1 . 2} x.x.x.x:$port) };
  (CON POST x.x.x.x:$port:literal) => { $crate::test::msg!(CON {1 . 3} x.x.x.x:$port) };
  (CON DELETE x.x.x.x:$port:literal) => { $crate::test::msg!(CON {1 . 4} x.x.x.x:$port) };
  (NON GET x.x.x.x:$port:literal) => { $crate::test::msg!(NON {1 . 1} x.x.x.x:$port) };
  (NON PUT x.x.x.x:$port:literal) => { $crate::test::msg!(NON {1 . 2} x.x.x.x:$port) };
  (NON POST x.x.x.x:$port:literal) => { $crate::test::msg!(NON {1 . 3} x.x.x.x:$port) };
  (NON DELETE x.x.x.x:$port:literal) => { $crate::test::msg!(NON {1 . 4} x.x.x.x:$port) };

  (CON {$c:literal . $d:literal} x.x.x.x:$port:literal) => {{
    $crate::test::msg!({toad_msg::Type::Con} {toad_msg::Code::new($c, $d)} x.x.x.x:$port)
  }};
  (NON {$c:literal . $d:literal} x.x.x.x:$port:literal) => {{
    $crate::test::msg!({toad_msg::Type::Non} {toad_msg::Code::new($c, $d)} x.x.x.x:$port)
  }};
  (ACK {$c:literal . $d:literal} x.x.x.x:$port:literal) => {{
    $crate::test::msg!({toad_msg::Type::Ack} {toad_msg::Code::new($c, $d)} x.x.x.x:$port)
  }};
  (ACK EMPTY x.x.x.x:$port:literal) => {{
    $crate::test::msg!({toad_msg::Type::Ack} {toad_msg::Code::new(0, 0)} x.x.x.x:$port)
  }};

  ({$ty:expr} {$code:expr} x.x.x.x:$port:literal) => {{
    use $crate::net::Addrd;
    use no_std_net::*;
    use toad_msg::*;

    let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(192, 168, 0, 1), $port));

    Addrd(test::Message {
      ver: Default::default(),
      ty: $ty,
      token: Token(Default::default()),
      code: $code,
      id: Id(0),
      opts: Default::default(),
      payload: Payload(Default::default()),
    }, addr)
  }};
}

pub use msg;

pub type Message = crate::platform::Message<Platform>;
pub type Snapshot = crate::platform::SnapshotForPlatform<Platform>;
pub type Req = crate::req::ReqForPlatform<Platform>;
pub type Resp = crate::resp::RespForPlatform<Platform>;

pub fn dummy_addr() -> SocketAddr {
  SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(192, 168, 0, 1), 8080))
}

pub fn dummy_addr_2() -> SocketAddr {
  SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(192, 168, 0, 2), 8080))
}

pub fn dummy_addr_3() -> SocketAddr {
  SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(192, 168, 0, 3), 8080))
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TimeoutState {
  Canceled,
  WillPanic,
}

pub struct Timeout {
  pub state: Arc<Mutex<TimeoutState>>,
  dur: Duration,
}

impl Timeout {
  pub fn new(dur: Duration) -> Self {
    Self { state: Arc::new(Mutex::new(TimeoutState::WillPanic)),
           dur }
  }

  pub fn cancel(state: Arc<Mutex<TimeoutState>>) {
    *state.lock().unwrap() = TimeoutState::Canceled;
  }

  pub fn wait(&self) {
    if self.state.lock().unwrap().deref() == &TimeoutState::Canceled {
      return;
    };

    thread::sleep(self.dur);
    if self.state.lock().unwrap().deref() == &TimeoutState::WillPanic {
      panic!("test timed out");
    } else {
      ()
    }
  }
}

/// Config implementor using mocks for clock and sock
pub type Platform = crate::platform::Alloc<ClockMock, SockMock>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ClockMock(pub Cell<u64>);

impl ClockMock {
  pub fn new() -> Self {
    Self(Cell::new(0))
  }

  pub fn set(&self, to: u64) {
    self.0.set(to);
  }

  pub fn instant(n: u64) -> Instant<Self> {
    Instant::new(n)
  }
}

impl embedded_time::Clock for ClockMock {
  type T = u64;

  const SCALING_FACTOR: Fraction = Fraction::new(1, 1_000_000);

  fn try_now(&self) -> Result<Instant<Self>, embedded_time::clock::Error> {
    Ok(Instant::new(self.0.get()))
  }
}

/// A mocked socket
#[derive(Debug)]
pub struct SockMock {
  /// Inbound bytes from remote sockets. Address represents the sender
  pub rx: Arc<Mutex<Vec<Addrd<Vec<u8>>>>>,
  /// Outbound bytes to remote sockets. Address represents the destination
  pub tx: Arc<Mutex<Vec<Addrd<Vec<u8>>>>>,
}

impl SockMock {
  pub fn new() -> Self {
    Self { rx: Default::default(),
           tx: Default::default() }
  }

  pub fn send_msg<P: platform::Platform>(rx: &Arc<Mutex<Vec<Addrd<Vec<u8>>>>>,
                                         msg: Addrd<platform::Message<P>>) {
    rx.lock()
      .unwrap()
      .push(msg.map(|msg| msg.try_into_bytes().unwrap()));
  }

  pub fn await_msg<P: platform::Platform>(addr: SocketAddr,
                                          tx: &Arc<Mutex<Vec<Addrd<Vec<u8>>>>>)
                                          -> platform::Message<P> {
    let attempt = || {
      tx.lock()
        .unwrap()
        .iter_mut()
        .find(|bytes| bytes.addr() == addr && !bytes.data().is_empty())
        .map(|Addrd(bytes, _)| {
          platform::Message::<P>::try_from_bytes(bytes.drain(..).collect::<Vec<_>>()).unwrap()
        })
    };

    loop {
      if let Some(msg) = attempt() {
        break msg;
      }
    }
  }
}

impl Socket for SockMock {
  type Error = Option<()>;

  fn recv(&self, buf: &mut [u8]) -> nb::Result<Addrd<usize>, Self::Error> {
    let mut rx = self.rx.lock().unwrap();

    if rx.is_empty() {
      return Err(nb::Error::WouldBlock);
    }

    let dgram = rx.drain(0..1).next().unwrap();

    dgram.data()
         .iter()
         .enumerate()
         .for_each(|(ix, byte)| buf[ix] = *byte);

    Ok(dgram.map(|bytes| bytes.len()))
  }

  fn send(&self, buf: Addrd<&[u8]>) -> nb::Result<(), Self::Error> {
    let mut vec = self.tx.lock().unwrap();
    vec.push(buf.map(Vec::from));
    Ok(())
  }

  fn join_multicast(&self, _: no_std_net::IpAddr) -> Result<(), Self::Error> {
    todo!()
  }

  fn bind_raw<A: no_std_net::ToSocketAddrs>(_: A) -> Result<Self, Self::Error> {
    Ok(Self::new())
  }

  fn peek(&self, _: &mut [u8]) -> nb::Result<Addrd<usize>, Self::Error> {
    todo!()
  }
}

#[test]
#[should_panic]
fn times_out() {
  let timeout = Timeout::new(Duration::from_millis(100));
  thread::spawn(|| thread::sleep(Duration::from_millis(110)));
  timeout.wait();
}

#[test]
fn doesnt_time_out() {
  let timeout = Timeout::new(Duration::from_secs(1));
  let state = timeout.state.clone();
  Timeout::cancel(state);
  timeout.wait();
}
