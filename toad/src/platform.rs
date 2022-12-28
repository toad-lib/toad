use core::fmt::Debug;

use embedded_time::Instant;
use no_std_net::SocketAddr;
#[cfg(feature = "alloc")]
use std_alloc::vec::Vec;
use toad_common::*;
use toad_msg::{Opt, OptNumber, Token, TryIntoBytes};

use crate::config::Config;
use crate::net::{Addrd, Socket};
use crate::step::Step;
use crate::time::Clock;
use crate::todo::String1Kb;

/// Default [`PlatformError`] implementation
#[derive(Debug)]
#[allow(missing_docs)]
pub enum Error<Step, Socket> {
  MessageToBytes(toad_msg::to_bytes::MessageToBytesError),
  Step(Step),
  Socket(Socket),
  Clock(embedded_time::clock::Error),
}

impl<Step, Socket> PlatformError<Step, Socket> for Error<Step, Socket> {
  fn msg_to_bytes(e: toad_msg::to_bytes::MessageToBytesError) -> Self {
    Self::MessageToBytes(e)
  }

  fn step(e: Step) -> Self {
    Self::Step(e)
  }

  fn socket(e: Socket) -> Self {
    Self::Socket(e)
  }

  fn clock(e: embedded_time::clock::Error) -> Self {
    Self::Clock(e)
  }
}

/// Errors that may be encountered during the CoAP lifecycle
pub trait PlatformError<StepError, SocketError>: Sized {
  /// Convert a [`toad_msg::to_bytes::MessageToBytesError`] to PlatformError
  fn msg_to_bytes(e: toad_msg::to_bytes::MessageToBytesError) -> Self;

  /// Convert a step error to PlatformError
  fn step(e: StepError) -> Self;

  /// Convert a socket error to PlatformError
  fn socket(e: SocketError) -> Self;

  /// Convert a clock error to PlatformError
  fn clock(e: embedded_time::clock::Error) -> Self;
}

/// The runtime component of the `Platform` abstraction
///
/// Uses [`PlatformTypes`], [`Steps`](Step), and [`PlatformError`] to create
/// an interface that covers the CoAP protocol.
///
/// # Type Arguments
/// * `Steps`
///   * The CoAP runtime, plus user code and potential extensions. The [`Step`] trait represents a type-level linked list of steps that, when all executed, add up to the CoAP runtime. For more information see the [`Step`] trait.
/// * [`Platform::Types`]
/// * [`Platform::Error`]
pub trait Platform<Steps: Step<Self::Types, PollReq = (), PollResp = ()>>: Default {
  /// See [`PlatformTypes`]
  type Types: PlatformTypes;

  /// Slot for any error type that impls [`PlatformError`].
  ///
  /// If no custom behavior is needed, [`self::Error`] is a sensible default.
  type Error: PlatformError<<Steps as Step<Self::Types>>::Error,
                            <<Self::Types as PlatformTypes>::Socket as Socket>::Error>;

  /// Take a snapshot of the platform's state right now,
  /// including the system time and datagrams currently
  /// in the network socket
  fn snapshot(&self) -> nb::Result<Snapshot<Self::Types>, Self::Error> {
    use embedded_time::Clock;

    self.socket()
        .poll()
        .map_err(Self::Error::socket)
        .map_err(nb::Error::Other)
        .and_then(|dgram: Option<_>| dgram.map(Ok).unwrap_or(Err(nb::Error::WouldBlock)))
        .and_then(|recvd_dgram| {
          self.clock()
              .try_now()
              .map_err(Self::Error::clock)
              .map_err(nb::Error::Other)
              .map(|time| Snapshot { recvd_dgram,
                                     config: self.config(),
                                     time })
        })
  }

  /// Poll for an incoming request, and pass it through `Steps`
  /// for processing.
  fn poll_req(&self) -> nb::Result<(), Self::Error> {
    let mut effects = <Self::Types as PlatformTypes>::Effects::default();
    self.snapshot().and_then(|snapshot| {
                     self.steps()
                         .poll_req(&snapshot, &mut effects)
                         .unwrap_or(Err(nb::Error::WouldBlock))
                         .map_err(|e: nb::Error<_>| e.map(Self::Error::step))
                   })
  }

  /// Poll for a response to a sent request, and pass it through `Steps`
  /// for processing.
  fn poll_resp(&self, token: Token, addr: SocketAddr) -> nb::Result<(), Self::Error> {
    let mut effects = <Self::Types as PlatformTypes>::Effects::default();
    self.snapshot().and_then(|snapshot| {
                     self.steps()
                         .poll_resp(&snapshot, &mut effects, token, addr)
                         .unwrap_or(Err(nb::Error::WouldBlock))
                         .map_err(|e: nb::Error<_>| e.map(Self::Error::step))
                   })
  }

  /// `toad` may occasionally emit tracing and logs by invoking this method.
  ///
  /// It's completely up to the Platform to handle them meaningfully (e.g. `println!`)
  fn log(&self, level: log::Level, msg: String1Kb) -> Result<(), Self::Error>;

  /// Send a [`toad_msg::Message`]
  fn send_msg(&self, mut addrd_msg: Addrd<Message<Self::Types>>) -> nb::Result<(), Self::Error> {
    type Dgram<P> = <<P as PlatformTypes>::Socket as Socket>::Dgram;

    self.snapshot()
        .try_perform(|snapshot| {
          self.steps()
              .before_message_sent(snapshot, &mut addrd_msg)
              .map_err(Self::Error::step)
              .map_err(nb::Error::Other)
        })
        .and_then(|snapshot| {
          addrd_msg.clone().fold(|msg, addr| {
                             msg.try_into_bytes::<Dgram<Self::Types>>()
                                .map_err(Self::Error::msg_to_bytes)
                                .map_err(nb::Error::Other)
                                .map(|bytes| (snapshot, Addrd(bytes, addr)))
                           })
        })
        .try_perform(|(_, addrd_bytes)| {
          self.socket()
              .send(addrd_bytes.as_ref().map(|s| s.as_ref()))
              .map_err(|e: nb::Error<_>| e.map(Self::Error::socket))
        })
        .try_perform(|(snapshot, _)| {
          self.steps()
              .on_message_sent(snapshot, &addrd_msg)
              .map_err(Self::Error::step)
              .map_err(nb::Error::Other)
        })
        .map(|_| ())
  }

  /// Execute an [`Effect`]
  fn exec_1(&self, effect: &Effect<Self::Types>) -> nb::Result<(), Self::Error> {
    match effect {
      | &Effect::Log(level, msg) => self.log(level, msg).map_err(nb::Error::Other),
      | &Effect::Send(_) => todo!(),
    }
  }

  /// Execute many [`Effect`]s
  ///
  /// Blocks on effects that yield `nb::WouldBlock`.
  ///
  /// If executing an effect errors, the erroring effect and all remaining effects are
  /// returned along with the error.
  fn exec_many(&self,
               effects: <Self::Types as PlatformTypes>::Effects)
               -> Result<(), (<Self::Types as PlatformTypes>::Effects, Self::Error)> {
    effects.into_iter()
           .fold(Ok(()), |so_far, eff| match so_far {
             | Ok(()) => nb::block!(self.exec_1(&eff)).map_err(|e| {
                           let mut effs: <Self::Types as PlatformTypes>::Effects =
                             Default::default();
                           effs.push(eff);
                           (effs, e)
                         }),
             | Err((mut effs, e)) => {
               effs.push(eff);
               Err((effs, e))
             },
           })
  }

  /// Copy of runtime behavior [`Config`] to be used
  ///
  /// Typically this will be a field access (`self.config`)
  fn config(&self) -> Config;

  /// Obtain a reference to [`Steps`](#type-arguments)
  ///
  /// Typically this will be a field access (`&self.steps`)
  fn steps(&self) -> &Steps;

  /// Obtain an immutable reference
  ///
  /// Typically this will be a field access (`&self.socket`)
  fn socket(&self) -> &<Self::Types as PlatformTypes>::Socket;

  /// Get a reference to the system clock
  ///
  /// Typically this will be a field access (`&self.clock`)
  fn clock(&self) -> &<Self::Types as PlatformTypes>::Clock;
}

/// toad configuration trait
pub trait PlatformTypes: Sized + 'static + core::fmt::Debug {
  /// What type should we use to store the message payloads?
  type MessagePayload: Array<Item = u8> + Clone + Debug + PartialEq + AppendCopy<u8>;

  /// What type should we use to store the option values?
  type MessageOptionBytes: Array<Item = u8> + 'static + Clone + Debug + PartialEq + AppendCopy<u8>;

  /// What type should we use to store the options?
  type MessageOptions: Array<Item = Opt<Self::MessageOptionBytes>> + Clone + Debug + PartialEq;

  /// What type should we use to keep track of options before serializing?
  type NumberedOptions: Array<Item = (OptNumber, Opt<Self::MessageOptionBytes>)>
    + Clone
    + Debug
    + PartialEq;

  /// What should we use to keep track of time?
  type Clock: Clock;

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
pub struct Snapshot<P: PlatformTypes> {
  /// The current system time at the start of the step pipe
  pub time: Instant<P::Clock>,

  /// A UDP datagram received from somewhere
  pub recvd_dgram: Addrd<<P::Socket as Socket>::Dgram>,

  /// Runtime config, includes many useful timings
  pub config: Config,
}

impl<P: PlatformTypes> Clone for Snapshot<P> {
  fn clone(&self) -> Self {
    Self { time: self.time,
           recvd_dgram: self.recvd_dgram.clone(),
           config: self.config }
  }
}

/// Used by [`Step`]s to deterministically communicate
/// to [`Platform`]s side-effects that they would like
/// to perform.
#[allow(missing_docs)]
pub enum Effect<P: PlatformTypes> {
  Send(Addrd<Message<P>>),
  Log(log::Level, String1Kb),
}

impl<P: PlatformTypes> Clone for Effect<P> {
  fn clone(&self) -> Self {
    match self {
      | Effect::Send(m) => Effect::Send(m.clone()),
      | Effect::Log(l, m) => Effect::Log(*l, *m),
    }
  }
}

impl<P: PlatformTypes> core::fmt::Debug for Effect<P> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    match self {
      | Self::Send(m) => f.debug_tuple("Send").field(m).finish(),
      | Self::Log(l, s) => f.debug_tuple("Log").field(l).field(s).finish(),
    }
  }
}

impl<P: PlatformTypes> PartialEq for Effect<P> {
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      | (Self::Send(a), Self::Send(b)) => a == b,
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
pub struct Retryable<P: PlatformTypes, T>(pub T, pub crate::retry::RetryTimer<P::Clock>);

impl<P: PlatformTypes, T> Retryable<P, T> {
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
impl<Clk: Clock + Debug + 'static, Sock: Socket + 'static> PlatformTypes for Alloc<Clk, Sock> {
  type MessagePayload = Vec<u8>;
  type MessageOptionBytes = Vec<u8>;
  type MessageOptions = Vec<Opt<Vec<u8>>>;
  type NumberedOptions = Vec<(OptNumber, Opt<Vec<u8>>)>;
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
  toad_msg::Message<<P as PlatformTypes>::MessagePayload, <P as PlatformTypes>::MessageOptions>;
