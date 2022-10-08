#![allow(dead_code)]

use ::core::cell::Cell;
use ::core::ops::Deref;
use ::core::time::Duration;
use ::std::sync::Mutex;
use ::std::thread;
use embedded_time::rate::Fraction;
use embedded_time::Instant;
use toad_msg::{TryFromBytes, TryIntoBytes};
use net::*;
use no_std_net::SocketAddr;
use std_alloc::sync::Arc;

use super::*;

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
pub type Config = crate::platform::Alloc<ClockMock, SockMock>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
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
