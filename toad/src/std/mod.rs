#![allow(clippy::many_single_char_names)]

use embedded_time::rate::Fraction;

/// Networking! woohoo!
pub mod net;
pub use net::*;
use toad_msg::{Opt, OptNumber};

use crate::{platform::Effect, step::Step};
use std::fmt::Debug;
use std::io;
use std::net::UdpSocket;

/// implementor of [`crate::platform::PlatformTypes`] for
/// platforms that support `std`.
#[derive(Clone, Copy, Debug)]
pub struct PlatformTypes;

impl crate::platform::PlatformTypes for PlatformTypes {
  type MessagePayload = Vec<u8>;
  type MessageOptionBytes = Vec<u8>;
  type MessageOptions = Vec<Opt<Vec<u8>>>;
  type NumberedOptions = Vec<(OptNumber, Opt<Vec<u8>>)>;
  type Clock = Clock;
  type Socket = UdpSocket;
  type Effects = Vec<Effect<Self>>;
}

impl<StepError, SocketError> crate::platform::PlatformError<StepError, SocketError> for io::Error where StepError: Debug, SocketError: Debug {
    fn msg_to_bytes(e: toad_msg::to_bytes::MessageToBytesError) -> Self {
      io::Error::new(io::ErrorKind::InvalidData, format!("{:?}", e))
    }

    fn step(e: StepError) -> Self {
      io::Error::new(io::ErrorKind::Other, format!("{:?}", e))
    }

    fn socket(e: SocketError) -> Self {
     io::Error::new(io::ErrorKind::Other, format!("{:?}", e))
    }

    fn clock(e: embedded_time::clock::Error) -> Self {
      io::Error::new(io::ErrorKind::Other, format!("{:?}", e))
    }
}

/// implementor of [`crate::platform::Platform`] for `std`
#[derive(Debug)]
pub struct Platform<Steps> {
  steps: Steps,
  config: crate::config::Config,
  socket: UdpSocket,
  clock: Clock,
}

impl<Steps> Platform<Steps> {
  /// Create a new std runtime
  pub fn try_new<A: std::net::ToSocketAddrs>(bind_to_addr: A, cfg: crate::config::Config) -> io::Result<Self> where Steps: Default {
    UdpSocket::bind(bind_to_addr).map(|socket|
    Self {
      steps: Steps::default(),
      config: cfg,
      socket,
      clock: Clock::new(),
    })
  }
}

impl<Steps> crate::platform::Platform<Steps> for Platform<Steps> where Steps: Step<PlatformTypes, PollReq = (), PollResp = ()> {
    type Types = PlatformTypes;
    type Error = io::Error;

    fn log(&self, level: log::Level, msg: crate::todo::String1Kb) -> Result<(), Self::Error> {
      log::log!(target: "toad", level, "{}", msg.as_ref());
      Ok(())
    }

    fn config(&self) -> crate::config::Config {
      self.config
    }

    fn steps(&self) -> &Steps {
      &self.steps
    }

    fn socket(&self) -> &UdpSocket {
      &self.socket
    }

    fn clock(&self) -> &Clock {
      &self.clock
    }
}

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

  // microseconds
  const SCALING_FACTOR: Fraction = Fraction::new(1, 1_000_000);

  fn try_now(&self) -> Result<embedded_time::Instant<Self>, embedded_time::clock::Error> {
    let now = std::time::Instant::now();
    let elapsed = now.duration_since(self.0);
    Ok(embedded_time::Instant::new(elapsed.as_micros() as u64))
  }
}
