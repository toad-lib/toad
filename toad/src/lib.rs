//! `toad` is a Rust CoAP implementation that aims to be:
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

// x-release-please-version
#![doc(html_root_url = "https://docs.rs/toad/0.12.1")]
// x-release-please-end
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

#[cfg(test)]
pub(crate) mod test;

pub(crate) mod logging;

/// Blocking rust CoAP client & server
pub mod blocking;

/// customizable retrying of fallible operations
pub mod retry;

/// responses
pub mod resp;

/// requests
pub mod req;

/// low-level coap behavior
pub mod core;

/// # CoAP core runtime
///
/// The core CoAP runtime is broken into discrete steps
/// that are mostly deterministic and therefore highly
/// testable.
///
/// Steps are expressed as types that impl a [`Step`](crate::step::Step) trait
/// which defines 2 flows: "poll for a request" and "poll for a response to a request i sent"
///
/// Steps are usually parameterized by 1 type; the Step that came before it.
///
/// This means that the entire CoAP runtime transparently describes what happens
/// when a message is received, and layers can be swapped or added at the end
/// without forking `toad`.
///
/// # Step demands
/// Steps demand 2 pieces of information:
///  - A snapshot of the system's state right now
///  - A mutable reference to a list of effectful actions to perform once all steps have run
///
/// The system state allows for all steps to have access to the same effectful information
/// e.g. system time, random number generation, incoming network messages
///
/// The list of Effects allows for steps to deterministically express the IO that it would
/// like to be performed, e.g. log to stdout or send network messages.
///
/// # Step philosophy
/// In general, steps aim to be as deterministic as possible. The obvious exception
/// to this is the mutable reference to `Effects`, but philosophically this can be
/// thought of as a performance-enhanced immutable list.
///
/// The effect of this is that each step can be thought of as a state machine, such that
/// if you send the same sequence of inputs you will always receive the same output.
///
/// For steps defined in `toad`, this philosophy will **always** be respected
///
/// If you are a `toad` user, this philosophy **may** be respected, but the implications
/// of performing IO in your steps (e.g. network requests) will not affect the runtime
/// in any way.
///
/// # Example
/// ```no_run
/// Bake<PourIntoCakeTin<MixEverything<MixDry<MixWet<GatherIngredients<Empty>>>>>>
/// ```
/// exploded:
/// ```no_run
/// Bake<
///   PourIntoCakeTin<
///     MixEverything<
///       MixDry<
///         MixWet<
///           GatherIngredients<
///             Empty
///           >
///         >
///       >
///     >
///   >
/// >
/// ```
pub mod step;

/// platform configuration
pub mod platform;

/// network abstractions
pub mod net;

/// time abstractions
pub mod time;

/// configuring runtime behavior
pub mod config;

/// `std`-only toad stuff
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub mod std;

mod option;

pub use option::{ContentFormat, ToCoapValue};

/// Helper constants and functions for creating multicast addresses
pub mod multicast {
  use no_std_net::{Ipv4Addr, SocketAddr, SocketAddrV4};

  /// IPv4 "All CoAP devices" multicast address.
  ///
  /// If using multicast to discover devices, it's recommended
  /// that you use this address with a port specific to your application.
  pub const ALL_COAP_DEVICES_IP: Ipv4Addr = Ipv4Addr::new(224, 0, 1, 187);

  /// Create a SocketAddr (IP + port) with the [`ALL_COAP_DEVICES_IP`] address
  ///
  /// If using multicast to discover devices, it's recommended
  /// that you use this address with a port specific to your application.
  pub const fn all_coap_devices(port: u16) -> SocketAddr {
    SocketAddr::V4(SocketAddrV4::new(ALL_COAP_DEVICES_IP, port))
  }
}

macro_rules! code {
  (rfc7252($section:literal) $name:ident = $c:literal.$d:literal) => {
    #[doc = toad_macros::rfc_7252_doc!($section)]
    #[allow(clippy::zero_prefixed_literal)]
    pub const $name: toad_msg::Code = toad_msg::Code::new($c, $d);
  };
  (rfc7252($section:literal) $name:ident = $newtype:tt($c:literal.$d:literal)) => {
    #[doc = toad_macros::rfc_7252_doc!($section)]
    #[allow(clippy::zero_prefixed_literal)]
    pub const $name: $newtype = $newtype(toad_msg::Code::new($c, $d));
  };
}

pub(crate) use code;
