use core::fmt::Debug;

use embedded_time::Clock;
use kwap_common::Array;
use kwap_msg::{Opt, OptNumber};
#[cfg(feature = "alloc")]
use std_alloc::vec::Vec;

use crate::net::Socket;

/// kwap configuration trait
pub trait Platform: Sized + 'static + core::fmt::Debug {
  /// What type should we use to store the message payloads?
  type MessagePayload: Array<Item = u8> + Clone + Debug;
  /// What type should we use to store the option values?
  type MessageOptionBytes: Array<Item = u8> + 'static + Clone + Debug;
  /// What type should we use to store the options?
  type MessageOptions: Array<Item = Opt<Self::MessageOptionBytes>> + Clone + Debug;

  /// What type should we use to keep track of options before serializing?
  type NumberedOptions: Array<Item = (OptNumber, Opt<Self::MessageOptionBytes>)> + Clone + Debug;

  /// What should we use to keep track of time?
  type Clock: Clock<T = u64>;

  /// What should we use for networking?
  type Socket: Socket;
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

/// Configures `kwap` to use `Vec` for collections,
/// `UdpSocket` for networking,
/// and [`crate::std::Clock`] for timing
///
/// ```
/// use kwap::platform::Std;
/// use kwap::req::Req;
///
/// Req::<Std>::get("192.168.0.1", 5683, "/hello");
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
impl<Clk: Clock<T = u64> + 'static, Sock: Socket + 'static> Platform for Alloc<Clk, Sock> {
  type MessagePayload = Vec<u8>;
  type MessageOptionBytes = Vec<u8>;
  type MessageOptions = Vec<Opt<Vec<u8>>>;
  type NumberedOptions = Vec<(OptNumber, Opt<Vec<u8>>)>;
  type Clock = Clk;
  type Socket = Sock;
}

/// Configures `kwap` to use `Vec` for collections,
/// `UdpSocket` for networking,
/// and [`crate::std::Clock`] for timing
///
/// ```
/// use kwap::platform::Std;
/// use kwap::req::Req;
///
/// Req::<Std>::get("192.168.0.1", 5683, "/hello");
/// ```
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub type Std = Alloc<crate::std::Clock, std::net::UdpSocket>;

/// Type alias using Config instead of explicit type parameters for [`kwap_msg::Message`]
pub type Message<P> = kwap_msg::Message<<P as Platform>::MessagePayload,
                                        <P as Platform>::MessageOptionBytes,
                                        <P as Platform>::MessageOptions>;
