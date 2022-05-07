#![allow(dead_code)]

use ::core::cell::Cell;
use ::core::ops::Deref;
use ::core::pin::Pin;
use ::core::time::Duration;
use ::std::sync::Mutex;
use embedded_time::rate::Fraction;
use embedded_time::Instant;
use kwap_msg::{TryFromBytes, TryIntoBytes};
use no_std_net::SocketAddr;
use socket::*;
use std_alloc::sync::Arc;

use super::*;

#[derive(PartialEq, Eq)]
enum TimeoutState {
  Canceled,
  WillPanic,
}

pub struct Timeout(Pin<Box<Mutex<TimeoutState>>>, Duration);

impl Timeout {
  pub fn new(dur: Duration) -> Self {
    Self(Box::pin(Mutex::new(TimeoutState::WillPanic)), dur)
  }

  pub fn eject_canceler(&self) -> Box<dyn FnOnce() + Send + 'static> {
    let canceler: Box<dyn FnOnce() + Send> = Box::new(|| *self.0.lock().unwrap() = TimeoutState::Canceled);
    unsafe { ::std::mem::transmute(canceler) }
  }

  pub fn wait(&self) {
    if self.0.lock().unwrap().deref() == &TimeoutState::Canceled {
      return;
    };

    ::std::thread::sleep(self.1);
    if self.0.lock().unwrap().deref() == &TimeoutState::WillPanic {
      panic!("test timed out");
    } else {
      ()
    }
  }
}

/// Config implementor using mocks for clock and sock
pub type Config = crate::config::Alloc<ClockMock, SockMock>;

pub struct ClockMock(pub Cell<u64>);

impl ClockMock {
  pub fn new() -> Self {
    Self(Cell::new(0))
  }

  pub fn set(&self, to: u64) {
    self.0.set(to);
  }
}

impl embedded_time::Clock for ClockMock {
  type T = u64;

  const SCALING_FACTOR: Fraction = Fraction::new(1, 1_000_000_000);

  fn try_now(&self) -> Result<Instant<Self>, embedded_time::clock::Error> {
    Ok(Instant::new(self.0.get()))
  }
}

/// A mocked socket
#[derive(Debug)]
pub struct SockMock {
  /// Inbound bytes from remote sockets. Address represents the sender
  pub rx: Arc<Mutex<Vec<Addressed<Vec<u8>>>>>,
  /// Outbound bytes to remote sockets. Address represents the destination
  pub tx: Arc<Mutex<Vec<Addressed<Vec<u8>>>>>,
}

impl SockMock {
  pub fn new() -> Self {
    Self { rx: Default::default(),
           tx: Default::default() }
  }

  pub fn send_msg<Cfg: config::Config>(rx: &Arc<Mutex<Vec<Addressed<Vec<u8>>>>>, msg: Addressed<config::Message<Cfg>>) {
    rx.lock().unwrap().push(msg.map(|msg| msg.try_into_bytes().unwrap()));
  }

  pub fn get_msg<Cfg: config::Config>(addr: SocketAddr,
                                      tx: &Arc<Mutex<Vec<Addressed<Vec<u8>>>>>)
                                      -> Option<config::Message<Cfg>> {
    tx.lock()
      .unwrap()
      .iter()
      .find(|bytes| bytes.addr() == addr)
      .and_then(|bytes| if bytes.data().is_empty() { None } else { Some(bytes) })
      .map(|bytes| config::Message::<Cfg>::try_from_bytes(bytes.data()).unwrap())
  }
}

impl Socket for SockMock {
  type Error = Option<()>;

  fn recv(&self, buf: &mut [u8]) -> nb::Result<Addressed<usize>, Self::Error> {
    let mut rx = self.rx.lock().unwrap();

    if rx.is_empty() {
      return Err(nb::Error::WouldBlock);
    }

    let dgram = rx.drain(0..1).next().unwrap();

    dgram.data().iter().enumerate().for_each(|(ix, byte)| buf[ix] = *byte);

    Ok(dgram.map(|bytes| bytes.len()))
  }

  fn send(&self, buf: Addressed<&[u8]>) -> nb::Result<(), Self::Error> {
    let mut vec = self.tx.lock().unwrap();
    vec.push(buf.map(Vec::from));
    Ok(())
  }
}

#[test]
#[should_panic]
fn times_out() {
  let timeout = Timeout::new(Duration::from_millis(100));
  ::std::thread::spawn(|| loop {});
  timeout.wait();
}

#[test]
fn doesnt_time_out() {
  let timeout = Timeout::new(Duration::from_secs(1));
  let cancel_timeout = timeout.eject_canceler();
  cancel_timeout();
  timeout.wait();
}
