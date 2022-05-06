//! `kwap` is a Rust CoAP implementation that aims to be:
//! - Platform-independent
//! - Extensible
//! - Approachable
//!
//! ## CoAP
//! CoAP is an application-level network protocol that copies the semantics of HTTP
//! to an environment conducive to **constrained** devices. (weak hardware, small battery capacity, etc.)
//!
//! This means that you can write and run two-way RESTful communication
//! between devices very similarly to the networking semantics you are
//! most likely very familiar with.
//!
//! ### Similarities to HTTP
//! CoAP has the same verbs and many of the same semantics as HTTP;
//! - GET, POST, PUT, DELETE
//! - Headers (renamed to [Options](https://datatracker.ietf.org/doc/html/rfc7252#section-5.10))
//! - Data format independent (via the [Content-Format](https://datatracker.ietf.org/doc/html/rfc7252#section-12.3) Option)
//! - [Response status codes](https://datatracker.ietf.org/doc/html/rfc7252#section-5.9)
//!
//! ### Differences from HTTP
//! - CoAP customarily sits on top of UDP (however the standard is [in the process of being adapted](https://tools.ietf.org/id/draft-ietf-core-coap-tcp-tls-11.html) to also run on TCP, like HTTP)
//! - Because UDP is a "connectionless" protocol, it offers no guarantee of "conversation" between traditional client and server roles. All the UDP transport layer gives you is a method to listen for messages thrown at you, and to throw messages at someone. Owing to this, CoAP machines are expected to perform both client and server roles (or more accurately, _sender_ and _receiver_ roles)
//! - While _classes_ of status codes are the same (Success 2xx -> 2.xx, Client error 4xx -> 4.xx, Server error 5xx -> 5.xx), the semantics of the individual response codes differ.

// docs
#![doc(html_root_url = "https://docs.rs/kwap/0.5.4")]
#![cfg_attr(any(docsrs, feature = "docs"), feature(doc_cfg))]
// -
// style
#![allow(clippy::unused_unit)]
// -
// deny
#![deny(missing_docs)]
#![deny(missing_debug_implementations)]
#![deny(missing_copy_implementations)]
#![cfg_attr(not(test), deny(unsafe_code))]
// -
// warnings
#![cfg_attr(not(test), warn(unreachable_pub))]
// -
// features
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc as std_alloc;

#[doc(hidden)]
pub mod todo;

/// Blocking CoAP client & server
pub mod blocking;

/// Customizable retrying of fallible operations
pub mod retry;

/// CoAP response messages
pub mod resp;

/// CoAP request messages
pub mod req;

/// CoAP client
pub mod core;

/// kwap configuration
pub mod config;

/// sockets
pub mod socket;

/// `std`-only kwap stuff
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub mod std;

mod option;
pub use option::{ContentFormat, ToCoapValue};

static mut ID: u16 = 0;
static mut TOKEN: u64 = 0;

// TODO(#79): Make token and ID generation use the Core's state and not mutable statics
fn generate_id() -> kwap_msg::Id {
  #[allow(unsafe_code)]
  unsafe {
    ID += 1;
    kwap_msg::Id(ID)
  }
}

// TODO(#79)
fn generate_token() -> kwap_msg::Token {
  #[allow(unsafe_code)]
  unsafe {
    TOKEN += 1;
    kwap_msg::Token(TOKEN.to_be_bytes().into())
  }
}

macro_rules! code {
  (rfc7252($section:literal) $name:ident = $c:literal.$d:literal) => {
    #[doc = kwap_macros::rfc_7252_doc!($section)]
    #[allow(clippy::zero_prefixed_literal)]
    pub const $name: kwap_msg::Code = kwap_msg::Code::new($c, $d);
  };
  (rfc7252($section:literal) $name:ident = $newtype:tt($c:literal.$d:literal)) => {
    #[doc = kwap_macros::rfc_7252_doc!($section)]
    #[allow(clippy::zero_prefixed_literal)]
    pub const $name: $newtype = $newtype(kwap_msg::Code::new($c, $d));
  };
}

pub(crate) use code;

#[cfg(test)]
pub(crate) mod test {
  use ::core::{cell::Cell, ops::Add};
  use ::core::ops::Deref;
  use ::core::pin::Pin;
  use ::core::time::Duration;
  use ::std::sync::Mutex;
  use embedded_time::rate::Fraction;
  use embedded_time::Instant;
  use kwap_msg::{TryFromBytes, TryIntoBytes};
  use no_std_net::{SocketAddr, ToSocketAddrs};
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

    pub fn get_msg<Cfg: config::Config>(addr: SocketAddr, tx: &Arc<Mutex<Vec<Addressed<Vec<u8>>>>>) -> Option<config::Message<Cfg>> {
      tx.lock().unwrap().iter().find(|bytes| bytes.addr() == addr)
          .and_then(|bytes| if bytes.data().is_empty() {
            None
          } else {Some(bytes)})
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
}
