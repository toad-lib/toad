use core::fmt::Debug;

use embedded_time::Instant;
use no_std_net::SocketAddr;
#[cfg(feature = "alloc")]
use std_alloc::{collections::BTreeMap, vec::Vec};
use toad_common::*;
use toad_msg::{Id, Opt, OptNumber, Token};

use crate::config::Config;
use crate::net::{Addrd, Socket};
use crate::time::{self, Clock, Stamped};
use crate::todo::{self, String1Kb};

/// toad configuration trait
pub trait Platform: Sized + 'static + core::fmt::Debug {
  /// What type should we use to store the message payloads?
  type MessagePayload: todo::MessagePayload;
  /// What type should we use to store the option values?
  type MessageOptionBytes: todo::MessageOptionValue;
  /// What type should we use to store the options?
  type MessageOptions: todo::MessageOptions<Self::MessageOptionBytes>;
  /// What type should we use to keep track of options before serializing?
  type NumberedOptions: todo::NumberedOptions<Self::MessageOptionBytes>;

  /// What type should we use to keep track of message IDs we've seen with a remote socket?
  type MessageIdHistory: Array<Item = Stamped<Self::Clock, Id>> + Clone + Debug;
  /// How do we track socket <> id histories?
  type MessageIdHistoryBySocket: Map<SocketAddr, Self::MessageIdHistory> + Clone + Debug;

  /// What type should we use to keep track of message Tokens we've seen with a remote socket?
  type MessageTokenHistory: Array<Item = Stamped<Self::Clock, Token>> + Clone + Debug;
  /// How do we track socket <> token histories?
  type MessageTokenHistoryBySocket: Map<SocketAddr, Self::MessageTokenHistory> + Clone + Debug;

  /// What should we use to keep track of time?
  type Clock: Clock;

  /// How will network datagrams be stored?
  type Dgram: crate::net::Dgram;

  /// What should we use for networking?
  type Socket: Socket;
}

/// [`Effect`] with generics filled in for some [`Platform`] P.
pub type EffectForPlatform<P> =
  Effect<<P as Platform>::MessagePayload, <P as Platform>::MessageOptions>;

/// [`Snapshot`] with generics filled in for some [`Platform`] P.
pub type SnapshotForPlatform<P> = Snapshot<<P as Platform>::Dgram, <P as Platform>::Clock>;

/// A snapshot of the system's state at a given moment
///
/// ```text
/// let Snapshot {time, recvd_dgram, ..} = snap;
/// ```
#[allow(missing_debug_implementations)]
#[non_exhaustive]
#[derive(Clone)]
pub struct Snapshot<Dgram, Clock: time::Clock> {
  /// The current system time at the start of the step pipe
  pub time: Instant<Clock>,

  /// A UDP datagram received from somewhere
  pub recvd_dgram: Addrd<Dgram>,

  /// Runtime config, includes many useful timings
  pub config: Config,
}

/// Side effects that platforms must support performing
#[derive(Clone, Debug, PartialEq)]
pub enum Effect<MessagePayload, MessageOptions> {
  /// Send a CoAP message to a remote address
  SendMessage(Addrd<toad_msg::Message<MessagePayload, MessageOptions>>),

  /// Log to some external log provider
  Log(log::Level, String1Kb),
}

/// Used to associate a value with a RetryTimer.
///
/// The value is usually used as the basis for some
/// fallible IO, e.g. `T` may be an outbound `Req` -
/// `Retryable` allows us to keep track of how many times
/// we've attempted to send this request and whether we
/// should consider it poisoned.
#[derive(Debug, Clone, Copy)]
pub struct Retryable<P: Platform, T>(pub T, pub crate::retry::RetryTimer<P::Clock>);

impl<P: Platform, T> Retryable<P, T> {
  /// Gets the data, discarding the retry timer
  pub fn unwrap(self) -> T {
    self.0
  }
}

/// Configures `toad` to use `Vec` for collections,
/// `UdpSocket` for networking, and [`crate::std::Clock`] for timing
#[cfg(feature = "alloc")]
#[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
#[derive(Copy)]
pub struct Alloc<Clk, Sock>(core::marker::PhantomData<(Clk, Sock)>)
  where Clk: Clock + 'static,
        Sock: Socket + 'static;

#[cfg(feature = "alloc")]
impl<Clk: Clock + 'static, Sock: Socket + 'static> core::fmt::Debug for Alloc<Clk, Sock> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "Alloc::<_, _>(_)")
  }
}

#[cfg(feature = "alloc")]
impl<Clk: Clock + 'static, Sock: Socket + 'static> Clone for Alloc<Clk, Sock> {
  fn clone(&self) -> Self {
    Self(Default::default())
  }
}

#[cfg(feature = "alloc")]
impl<Clk: Clock + Debug + 'static, Sock: Socket + 'static> Platform for Alloc<Clk, Sock> {
  type MessagePayload = Vec<u8>;
  type MessageOptionBytes = Vec<u8>;
  type MessageOptions = Vec<Opt<Vec<u8>>>;
  type MessageIdHistory = Vec<Stamped<Self::Clock, Id>>;
  type MessageTokenHistory = Vec<Stamped<Self::Clock, Token>>;
  type MessageIdHistoryBySocket = BTreeMap<SocketAddr, Self::MessageIdHistory>;
  type MessageTokenHistoryBySocket = BTreeMap<SocketAddr, Self::MessageTokenHistory>;
  type NumberedOptions = Vec<(OptNumber, Opt<Vec<u8>>)>;
  type Dgram = Vec<u8>;
  type Clock = Clk;
  type Socket = Sock;
}

/// Configures `toad` to use `Vec` for collections,
/// `UdpSocket` for networking,
/// and [`crate::std::Clock`] for timing
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub type Std = Alloc<crate::std::Clock, std::net::UdpSocket>;

/// [`Std`] secured with DTLS
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub type StdSecure = Alloc<crate::std::Clock, crate::std::net::SecureUdpSocket>;

/// [`toad_msg::Message`] shorthand using Platform types
pub type Message<P> =
  toad_msg::Message<<P as Platform>::MessagePayload, <P as Platform>::MessageOptions>;
