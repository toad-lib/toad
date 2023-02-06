#![allow(clippy::many_single_char_names)]

use embedded_time::rate::Fraction;

/// Networking! woohoo!
pub mod net;
use core::marker::PhantomData;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::io;

use dtls::sealed::Security;
pub use net::*;
use toad_msg::{Opt, OptNumber, OptValue};

use crate::net::{Addrd, Socket};
use crate::platform::{Effect, PlatformError};
use crate::req::Req;
use crate::resp::Resp;
use crate::step::Step;

/// Enable / Disable DTLS with types
pub mod dtls {
  use std::net::UdpSocket;

  use sealed::Security;

  use super::SecureUdpSocket;

  pub(super) mod sealed {
    use core::fmt::Debug;

    /// Whether or not DTLS enabled
    ///
    /// # Implementors
    pub trait Security: 'static + Debug {
      type Socket: crate::net::Socket;
    }
  }

  /// ZST marker for enabling DTLS
  #[derive(Debug, Clone, Copy)]
  pub struct Y;

  /// ZST marker for disabling DTLS
  #[derive(Debug, Clone, Copy)]
  pub struct N;

  impl Security for Y {
    type Socket = SecureUdpSocket;
  }

  impl Security for N {
    type Socket = UdpSocket;
  }
}

/// implementor of [`crate::platform::PlatformTypes`] for
/// platforms that support `std`.
#[derive(Clone, Copy, Debug)]
pub struct PlatformTypes<Sec>(PhantomData<Sec>) where Sec: Security;

impl<Sec> crate::platform::PlatformTypes for PlatformTypes<Sec> where Sec: Security
{
  type MessagePayload = Vec<u8>;
  type MessageOptionBytes = Vec<u8>;
  type MessageOptions = BTreeMap<OptNumber, Vec<OptValue<Vec<u8>>>>;
  type Clock = Clock;
  type Socket = Sec::Socket;
  type Effects = Vec<Effect<Self>>;
}

impl<StepError, SocketError> PlatformError<StepError, SocketError> for io::Error
  where StepError: Debug,
        SocketError: Debug
{
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
pub struct Platform<Sec, Steps>
  where Sec: Security
{
  steps: Steps,
  config: crate::config::Config,
  socket: Sec::Socket,
  clock: Clock,
}

impl<Sec, Steps> Platform<Sec, Steps>
  where Sec: Security,
        Steps: Step<PlatformTypes<Sec>,
                    PollReq = Addrd<Req<PlatformTypes<Sec>>>,
                    PollResp = Addrd<Resp<PlatformTypes<Sec>>>>
{
  /// Create a new std runtime
  pub fn try_new<A: std::net::ToSocketAddrs>(addr: A,
                                             cfg: crate::config::Config)
                                             -> io::Result<Self>
    where Steps: Default
  {
    fn first_addr<A_: std::net::ToSocketAddrs>(a: A_) -> io::Result<std::net::SocketAddr> {
      let yielded_no_addrs = || {
        io::Error::new(io::ErrorKind::InvalidInput,
                       "socket addr yielded 0 addresses")
      };

      a.to_socket_addrs()
       .and_then(|mut a| a.next().ok_or_else(yielded_no_addrs))
    }

    let socket_error =
      <io::Error as PlatformError<Steps::Error, <Sec::Socket as Socket>::Error>>::socket;

    first_addr(addr).map(|a| {
                      use net::convert::{no_std, std};
                      no_std::SockAddr::from(std::SockAddr(a)).0
                    })
                    .and_then(|a| Sec::Socket::bind(a).map_err(socket_error))
                    .map(|socket| Self { steps: Steps::default(),
                                         config: cfg,
                                         socket,
                                         clock: Clock::new() })
  }
}

impl<Sec, Steps> crate::platform::Platform<Steps> for Platform<Sec, Steps>
  where Sec: Security,
        Steps: Step<PlatformTypes<Sec>,
                    PollReq = Addrd<Req<PlatformTypes<Sec>>>,
                    PollResp = Addrd<Resp<PlatformTypes<Sec>>>>
{
  type Types = PlatformTypes<Sec>;
  type Error = io::Error;

  fn log(&self, level: log::Level, msg: crate::todo::String1Kb) -> Result<(), Self::Error> {
    log::log!(target: "toad", level, "{}", msg.as_str());
    Ok(())
  }

  fn config(&self) -> crate::config::Config {
    self.config
  }

  fn steps(&self) -> &Steps {
    &self.steps
  }

  fn socket(&self) -> &Sec::Socket {
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
