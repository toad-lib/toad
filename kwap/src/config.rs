use core::fmt::Debug;
use std::marker::PhantomData;
#[cfg(not(feature = "no_std"))]
use std::vec::Vec;

use embedded_time::Clock;
use kwap_common::Array;
use kwap_msg::{Opt, OptNumber};
#[cfg(all(feature = "no_std", feature = "alloc"))]
use std_alloc::vec::Vec;

use crate::core::event::Event;
use crate::socket::Socket;

/// Configures `kwap` to use `Vec` for collections,
/// `UdpSocket` for networking,
/// and [`crate::std::Clock`] for timing
///
/// ```
/// use kwap::config::Std;
/// use kwap::req::Req;
///
/// Req::<Std>::get("192.168.0.1", 5683, "/hello");
/// ```
#[cfg(feature = "alloc")]
#[derive(Debug, Copy)]
pub struct Alloc<Clk, Sock>(PhantomData<(Clk, Sock)>)
  where Clk: Clock<T = u64> + core::fmt::Debug + 'static,
        Sock: Socket + 'static + core::fmt::Debug;

impl<Clk: Clock<T = u64> + 'static + core::fmt::Debug, Sock: Socket + 'static + core::fmt::Debug> Clone for Alloc<Clk, Sock> {
  fn clone(&self) -> Self {
    Self(Default::default())
  }
}

impl<Clk: Clock<T = u64> + 'static + core::fmt::Debug, Sock: Socket + 'static + core::fmt::Debug> Config
  for Alloc<Clk, Sock>
{
  type PayloadBuffer = Vec<u8>;
  type OptBytes = Vec<u8>;
  type Opts = Vec<Opt<Vec<u8>>>;
  type OptNumbers = Vec<(OptNumber, Opt<Vec<u8>>)>;
  type Events = Vec<Event<Self>>;
  type Clock = Clk;
  type Socket = Sock;
}

/// Configures `kwap` to use `Vec` for collections,
/// `UdpSocket` for networking,
/// and [`crate::std::Clock`] for timing
///
/// ```
/// use kwap::config::Std;
/// use kwap::req::Req;
///
/// Req::<Std>::get("192.168.0.1", 5683, "/hello");
/// ```
#[cfg(not(feature = "no_std"))]
pub type Std = Alloc<crate::std::Clock, std::net::UdpSocket>;

/// kwap configuration trait
pub trait Config: Sized + 'static + core::fmt::Debug {
  /// What type should we use to store the message payloads?
  type PayloadBuffer: Array<Item = u8> + Clone + Debug;
  /// What type should we use to store the option values?
  type OptBytes: Array<Item = u8> + 'static + Clone + Debug;
  /// What type should we use to store the options?
  type Opts: Array<Item = Opt<Self::OptBytes>> + Clone + Debug;

  /// What type should we use to keep track of options before serializing?
  type OptNumbers: Array<Item = (OptNumber, Opt<Self::OptBytes>)> + Clone + Debug;

  /// What type should we use to store events?
  type Events: Array<Item = Event<Self>>;

  /// What should we use to keep track of time?
  type Clock: embedded_time::Clock<T = u64>;

  /// What should we use for networking?
  type Socket: crate::socket::Socket;
}

/// Type alias using Config instead of explicit type parameters for [`kwap_msg::Message`]
pub type Message<Cfg> =
  kwap_msg::Message<<Cfg as Config>::PayloadBuffer, <Cfg as Config>::OptBytes, <Cfg as Config>::Opts>;
