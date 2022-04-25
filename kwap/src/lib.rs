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

#![doc(html_root_url = "https://docs.rs/kwap/0.4.0")]
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(test),
            deny(missing_debug_implementations, unsafe_code, missing_copy_implementations))]
#![cfg_attr(not(test), warn(unreachable_pub,))]
#![cfg_attr(any(docsrs, feature = "docs"), feature(doc_cfg))]
#![deny(missing_docs)]
// - prefer explicit effectful statements that and in a () expr
// - prefer `fn foo() -> ()` to `fn foo()`
#![allow(clippy::unused_unit)]

#[cfg(feature = "alloc")]
extern crate alloc as std_alloc;

mod util;

/// Blocking CoAP client & server
pub mod blocking;

/// Customizable retrying of fallible operations
pub mod retry;

pub(crate) mod result_ext;

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

fn generate_id() -> kwap_msg::Id {
  // TEMPORARY
  // TODO: replace with long-living Client or Endpoint structure
  #[allow(unsafe_code)]
  unsafe {
    ID += 1;
    kwap_msg::Id(ID)
  }
}

fn generate_token() -> kwap_msg::Token {
  // TEMPORARY
  // TODO: replace with long-living Client or Endpoint structure
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
  use no_std_net::{SocketAddr, ToSocketAddrs};
  use socket::*;

  use super::*;

  /// A mocked socket
  #[derive(Clone, Debug, Default)]
  pub struct TubeSock {
    pub addr: Option<SocketAddr>,
    pub rx: Vec<u8>,
    pub tx: Vec<u8>,
  }

  impl TubeSock {
    pub fn new() -> Self {
      Self { addr: None,
             rx: Default::default(),
             tx: Default::default() }
    }

    pub fn init(addr: SocketAddr, rx: Vec<u8>) -> Self {
      let mut me = Self::new();
      me.addr = Some(addr);
      me.rx = rx;
      me
    }
  }

  impl Socket for TubeSock {
    type Error = Option<()>;

    fn connect<A: ToSocketAddrs>(&mut self, a: A) -> Result<(), Self::Error> {
      self.addr = a.to_socket_addrs().unwrap().next();
      Ok(())
    }

    fn recv(&self, buf: &mut [u8]) -> nb::Result<(usize, SocketAddr), Self::Error> {
      if self.addr.is_none() || self.rx.is_empty() {
        println!("TubeSock recv invoked without sending first");
        return Err(nb::Error::WouldBlock);
      }

      let n = self.rx.len();
      let vec = &self.rx as *const _ as *mut Vec<u8>;
      unsafe {
        vec.as_mut()
           .unwrap()
           .drain(..)
           .enumerate()
           .for_each(|(ix, el)| buf[ix] = el);
      }

      Ok((n, self.addr.unwrap()))
    }

    fn send(&self, buf: &[u8]) -> nb::Result<(), Self::Error> {
      let vec = &self.tx as *const _ as *mut Vec<u8>;
      unsafe {
        *vec = buf.iter().copied().collect();
      }
      Ok(())
    }
  }
}
