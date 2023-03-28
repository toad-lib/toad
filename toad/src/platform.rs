use core::fmt::Debug;

use ::toad_msg::{Id, OptNumber, OptValue, OptionMap, Token, TryIntoBytes};
use embedded_time::Instant;
use naan::prelude::MonadOnce;
use no_std_net::SocketAddr;
#[cfg(feature = "alloc")]
use std_alloc::vec::Vec;
use toad_array::{AppendCopy, Array};

use crate::config::Config;
use crate::net::{Addrd, Socket};
use crate::req::Req;
use crate::resp::Resp;
use crate::step::Step;
use crate::time::Clock;
use crate::todo::String;

/// Default [`PlatformError`] implementation
#[derive(Debug)]
#[allow(missing_docs)]
pub enum Error<Step, Socket> {
  MessageToBytes(::toad_msg::to_bytes::MessageToBytesError),
  Step(Step),
  Socket(Socket),
  Clock(embedded_time::clock::Error),
}

impl<Step, Socket> PlatformError<Step, Socket> for Error<Step, Socket>
  where Step: core::fmt::Debug,
        Socket: core::fmt::Debug
{
  fn msg_to_bytes(e: ::toad_msg::to_bytes::MessageToBytesError) -> Self {
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
pub trait PlatformError<StepError, SocketError>: Sized + core::fmt::Debug {
  /// Convert a [`toad_msg::to_bytes::MessageToBytesError`] to PlatformError
  fn msg_to_bytes(e: ::toad_msg::to_bytes::MessageToBytesError) -> Self;

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
pub trait Platform<Steps>
  where Steps:
          Step<Self::Types, PollReq = Addrd<Req<Self::Types>>, PollResp = Addrd<Resp<Self::Types>>>
{
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
  fn snapshot(&self) -> Result<Snapshot<Self::Types>, Self::Error> {
    use embedded_time::Clock;

    self.socket()
        .poll()
        .map_err(Self::Error::socket)
        .and_then(|recvd_dgram| {
          self.clock()
              .try_now()
              .map_err(Self::Error::clock)
              .map(|time| Snapshot { recvd_dgram,
                                     config: self.config(),
                                     time })
        })
  }

  /// Poll for an incoming request, and pass it through `Steps`
  /// for processing.
  fn poll_req(&self) -> nb::Result<Addrd<Req<Self::Types>>, Self::Error> {
    let mut effects = <Self::Types as PlatformTypes>::Effects::default();
    let res = self.snapshot()
                  .map_err(nb::Error::Other)
                  .and_then(|snapshot| {
                    self.steps()
                        .poll_req(&snapshot, &mut effects)
                        .unwrap_or(Err(nb::Error::WouldBlock))
                        .map_err(|e: nb::Error<_>| e.map(Self::Error::step))
                  });

    // NOTE: exec effects even if the above blocks
    self.exec_many(effects)
        .map_err(|(_, e)| e)
        .map_err(nb::Error::Other)?;

    res
  }

  /// Notify Observe subscribers that a new representation of the resource
  /// at `path` is available
  fn notify<P>(&self, path: P) -> Result<(), Self::Error>
    where P: AsRef<str> + Clone
  {
    let mut effects = <Self::Types as PlatformTypes>::Effects::default();
    self.steps()
        .notify(path, &mut effects)
        .map_err(Self::Error::step)?;

    self.exec_many(effects).map_err(|(_, e)| e)
  }

  /// Poll for a response to a sent request, and pass it through `Steps`
  /// for processing.
  fn poll_resp(&self,
               token: Token,
               addr: SocketAddr)
               -> nb::Result<Addrd<Resp<Self::Types>>, Self::Error> {
    let mut effects = <Self::Types as PlatformTypes>::Effects::default();
    let res = self.snapshot()
                  .map_err(nb::Error::Other)
                  .and_then(|snapshot| {
                    self.steps()
                        .poll_resp(&snapshot, &mut effects, token, addr)
                        .unwrap_or(Err(nb::Error::WouldBlock))
                        .map_err(|e: nb::Error<_>| e.map(Self::Error::step))
                  });

    // NOTE: exec effects even if the above blocks
    self.exec_many(effects)
        .map_err(|(_, e)| e)
        .map_err(nb::Error::Other)?;

    res
  }

  /// `toad` may occasionally emit tracing and logs by invoking this method.
  ///
  /// It's completely up to the Platform to handle them meaningfully (e.g. `println!`)
  fn log(&self, level: log::Level, msg: String<1000>) -> Result<(), Self::Error>;

  /// Send a [`toad_msg::Message`]
  fn send_msg(&self,
              mut addrd_msg: Addrd<self::toad_msg::Message<Self::Types>>)
              -> nb::Result<(Id, Token), Self::Error> {
    type Dgram<P> = <<P as PlatformTypes>::Socket as Socket>::Dgram;

    let mut effs = <Self::Types as PlatformTypes>::Effects::default();
    let mut on_message_sent_effs = <Self::Types as PlatformTypes>::Effects::default();

    self.snapshot()
        .discard(|snapshot: &Snapshot<Self::Types>| {
          self.steps()
              .before_message_sent(snapshot, &mut effs, &mut addrd_msg)
              .map_err(Self::Error::step)
        })
        .discard(|_: &Snapshot<Self::Types>| self.exec_many(effs).map_err(|(_, e)| e))
        .and_then(|snapshot| {
          addrd_msg.clone().fold(|msg, addr| {
                             let (id, token) = (msg.id, msg.token);
                             msg.try_into_bytes::<Dgram<Self::Types>>()
                                .map_err(Self::Error::msg_to_bytes)
                                .map(|bytes| (id, token, snapshot, Addrd(bytes, addr)))
                           })
        })
        .map_err(nb::Error::Other)
        .discard(|(_, _, _, addrd_bytes): &(_, _, _, Addrd<<<Self::Types as PlatformTypes>::Socket as Socket>::Dgram>)| {
          self.socket()
              .send(addrd_bytes.as_ref().map(|s| s.as_ref()))
              .map_err(|e: nb::Error<_>| e.map(Self::Error::socket))
        })
        .discard(|(_, _, snapshot, _): &(_, _, Snapshot<<Self as Platform<Steps>>::Types>, _)| {
          self.steps()
              .on_message_sent(snapshot, &mut on_message_sent_effs, &addrd_msg)
              .map_err(Self::Error::step)
              .map_err(nb::Error::Other)
        })
        .discard(|_: &(_, _, _, _)| self.exec_many(on_message_sent_effs).map_err(|(_, e)| e).map_err(nb::Error::Other))
        .map(|(id, token, _, _)| (id, token))
  }

  /// Execute an [`Effect`]
  fn exec_1(&self, effect: &Effect<Self::Types>) -> nb::Result<(), Self::Error> {
    match effect {
      | &Effect::Log(level, msg) => self.log(level, msg).map_err(nb::Error::Other),
      // TODO(orion): remove this clone as soon as `TryIntoBytes`
      // requires &msg not owned msg
      | &Effect::Send(ref msg) => self.send_msg(msg.clone()).map(|_| ()),
      | &Effect::Nop => Ok(()),
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

  /// `OptionMap::OptValues`
  type MessageOptionMapOptionValues: Array<Item = OptValue<Self::MessageOptionBytes>>
    + Clone
    + PartialEq
    + Debug;

  /// What type should we use to store the options?
  type MessageOptions: OptionMap<OptValues = Self::MessageOptionMapOptionValues, OptValue = Self::MessageOptionBytes>
    + Clone
    + Debug
    + PartialEq;

  /// What should we use to keep track of time?
  type Clock: Clock;

  /// What should we use for networking?
  type Socket: Socket;

  /// How will we store a sequence of effects to perform?
  type Effects: Array<Item = Effect<Self>> + core::fmt::Debug;
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
  pub recvd_dgram: Option<Addrd<<P::Socket as Socket>::Dgram>>,

  /// Runtime config, includes many useful timings
  pub config: Config,
}

impl<P: PlatformTypes> core::fmt::Debug for Snapshot<P> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_struct("Snapshot")
     .field("time", &self.time)
     .field("recvd_dgram", &self.recvd_dgram)
     .field("config", &self.config)
     .finish()
  }
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
pub enum Effect<P>
  where P: PlatformTypes
{
  Send(Addrd<self::toad_msg::Message<P>>),
  Log(log::Level, String<1000>),
  Nop,
}

impl<P> Effect<P> where P: PlatformTypes
{
  /// Is this [`Effect::Send`]?
  pub fn is_send(&self) -> bool {
    self.get_send().is_some()
  }

  /// If this is [`Effect::Send`], yields a reference to the message
  pub fn get_send(&self) -> Option<&Addrd<self::toad_msg::Message<P>>> {
    match self {
      | Self::Send(r) => Some(r),
      | _ => None,
    }
  }
}

impl<P> Default for Effect<P> where P: PlatformTypes
{
  fn default() -> Self {
    Self::Nop
  }
}

impl<P: PlatformTypes> Clone for Effect<P> {
  fn clone(&self) -> Self {
    match self {
      | Effect::Send(m) => Effect::Send(m.clone()),
      | Effect::Log(l, m) => Effect::Log(*l, *m),
      | Effect::Nop => Effect::Nop,
    }
  }
}

impl<P: PlatformTypes> core::fmt::Debug for Effect<P> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    match self {
      | Self::Send(m) => f.debug_tuple("Send").field(m).finish(),
      | Self::Log(l, s) => f.debug_tuple("Log").field(l).field(s).finish(),
      | Self::Nop => f.debug_tuple("Nop").finish(),
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
/// but you need to provide [`Clock`] and [`Socket`]
/// implementations.
#[cfg(feature = "alloc")]
#[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
#[derive(Copy)]
pub struct Alloc<Clk, Sock>(core::marker::PhantomData<(Clk, Sock)>)
  where Clk: Clock + 'static,
        Sock: Socket + 'static;

#[cfg(feature = "alloc")]
impl<Clk: Clock + 'static, Sock: Socket + 'static> core::fmt::Debug for Alloc<Clk, Sock> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "Alloc::<_, _>")
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
  type MessageOptionMapOptionValues = Vec<OptValue<Vec<u8>>>;
  type MessageOptions = std_alloc::collections::BTreeMap<OptNumber, Vec<OptValue<Vec<u8>>>>;
  type Clock = Clk;
  type Socket = Sock;
  type Effects = Vec<Effect<Self>>;
}

#[deprecated = "use `toad::platform::toad_msg::Message`"]
pub use self::toad_msg::Message;

/// Aliases that fill in verbose type arguments with
/// [`PlatformTypes`]
#[allow(missing_docs)]
pub mod toad_msg {
  use super::*;

  pub type Message<P> = ::toad_msg::Message<Payload<P>, opt::Map<P>>;
  pub type Payload<P> = <P as PlatformTypes>::MessagePayload;

  pub mod opt {
    use super::*;

    pub type Map<P> = <P as PlatformTypes>::MessageOptions;
    pub type Opt<P> = ::toad_msg::Opt<Bytes<P>>;
    pub type Bytes<P> = <Map<P> as ::toad_msg::OptionMap>::OptValue;
    pub type OptValue<P> = ::toad_msg::OptValue<Bytes<P>>;
    pub type SetError<P> =
      ::toad_msg::SetOptionError<::toad_msg::OptValue<Bytes<P>>, <Map<P> as OptionMap>::OptValues>;
  }
}
