use core::fmt::Debug;

use embedded_time::{Clock, Instant};
use no_std_net::SocketAddr;
#[cfg(feature = "alloc")]
use std_alloc::{collections::BTreeMap, vec::Vec};
use toad_common::*;
use toad_msg::{Id, Opt, OptNumber, Token};

use crate::net::{Addrd, Socket};
use crate::time::Stamped;
use crate::todo::String1Kb;

/// toad configuration trait
pub trait Platform: Sized + 'static + core::fmt::Debug {
  /// What type should we use to store the message payloads?
  type MessagePayload: Array<Item = u8> + Clone + Debug + PartialEq + AppendCopy<u8>;
  /// What type should we use to store the option values?
  type MessageOptionBytes: Array<Item = u8> + 'static + Clone + Debug + PartialEq + AppendCopy<u8>;
  /// What type should we use to store the options?
  type MessageOptions: Array<Item = Opt<Self::MessageOptionBytes>> + Clone + Debug + PartialEq;

  /// What type should we use to keep track of message IDs we've seen with a remote socket?
  type MessageIdHistory: Array<Item = Stamped<Self::Clock, Id>> + Clone + Debug;
  /// How do we track socket <> id histories?
  type MessageIdHistoryBySocket: Map<SocketAddr, Self::MessageIdHistory> + Clone + Debug;

  /// What type should we use to keep track of message Tokens we've seen with a remote socket?
  type MessageTokenHistory: Array<Item = Stamped<Self::Clock, Token>> + Clone + Debug;
  /// How do we track socket <> token histories?
  type MessageTokenHistoryBySocket: Map<SocketAddr, Self::MessageTokenHistory> + Clone + Debug;

  /// What type should we use to keep track of options before serializing?
  type NumberedOptions: Array<Item = (OptNumber, Opt<Self::MessageOptionBytes>)>
    + Clone
    + Debug
    + PartialEq;

  /// What should we use to keep track of time?
  type Clock: Clock<T = u64>;

  /// How will network datagrams be stored?
  type Dgram: Array<Item = u8> + AsRef<[u8]> + Clone + Debug + PartialEq;

  /// What should we use for networking?
  type Socket: Socket;

  /// How will we store a sequence of effects to perform?
  type Effects: Array<Item = Effect<Self>>;
}

/// A snapshot of the system's state at a given moment
///
/// ```text
/// let Snapshot {time, recvd_dgram, ..} = snap;
/// ```
#[allow(missing_debug_implementations)]
#[non_exhaustive]
pub struct Snapshot<P: Platform> {
  /// The current system time at the start of the step pipe
  pub time: Instant<P::Clock>,

  /// A UDP datagram received from somewhere
  pub recvd_dgram: Addrd<P::Dgram>,
}

impl<P: Platform> Snapshot<P> {
  /// Create a snapshot
  pub fn new(time: Instant<P::Clock>, recvd_dgram: Addrd<P::Dgram>) -> Self {
    Self { time, recvd_dgram }
  }
}

impl<P: Platform> Clone for Snapshot<P> {
  fn clone(&self) -> Self {
    Self::new(self.time, self.recvd_dgram.clone())
  }
}

/// Side effects that platforms must support performing
pub enum Effect<P: Platform> {
  /// Send a UDP message to a remote address
  SendDgram(Addrd<P::Dgram>),

  /// Log to some external log provider
  Log(log::Level, String1Kb),
}

impl<P: Platform> Clone for Effect<P> {
  fn clone(&self) -> Self {
    match self {
      | Effect::SendDgram(a) => Effect::SendDgram(a.clone()),
      | Effect::Log(l, m) => Effect::Log(*l, *m),
    }
  }
}

impl<P: Platform> core::fmt::Debug for Effect<P> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    match self {
      | Self::SendDgram(a) => f.debug_tuple("SendDgram").field(a).finish(),
      | Self::Log(l, s) => f.debug_tuple("Log").field(l).field(s).finish(),
    }
  }
}

impl<P: Platform> PartialEq for Effect<P> {
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      | (Self::SendDgram(a), Self::SendDgram(b)) => a == b,
      | (Self::Log(al, am), Self::Log(bl, bm)) => al == bl && am == bm,
      | _ => false,
    }
  }
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
///
/// ```
/// use toad::platform::Std;
/// use toad::req::Req;
///
/// Req::<Std>::get("192.168.0.1:5683".parse().unwrap(), "/hello");
/// ```
#[cfg(feature = "alloc")]
#[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
#[derive(Copy)]
pub struct Alloc<Clk, Sock>(core::marker::PhantomData<(Clk, Sock)>)
  where Clk: Clock<T = u64> + 'static,
        Sock: Socket + 'static;

#[cfg(feature = "alloc")]
impl<Clk: Clock<T = u64> + 'static, Sock: Socket + 'static> core::fmt::Debug for Alloc<Clk, Sock> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "Alloc::<_, _>(_)")
  }
}

#[cfg(feature = "alloc")]
impl<Clk: Clock<T = u64> + 'static, Sock: Socket + 'static> Clone for Alloc<Clk, Sock> {
  fn clone(&self) -> Self {
    Self(Default::default())
  }
}

#[cfg(feature = "alloc")]
impl<Clk: Clock<T = u64> + Debug + 'static, Sock: Socket + 'static> Platform for Alloc<Clk, Sock> {
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
  type Effects = Vec<Effect<Self>>;
}

/// Configures `toad` to use `Vec` for collections,
/// `UdpSocket` for networking,
/// and [`crate::std::Clock`] for timing
///
/// ```
/// use toad::platform::Std;
/// use toad::req::Req;
///
/// Req::<Std>::get("192.168.0.1:5683".parse().unwrap(), "/hello");
/// ```
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
