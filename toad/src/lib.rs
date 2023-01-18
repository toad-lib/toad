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

/// customizable retrying of fallible operations
pub mod retry;

/// responses
pub mod resp;

/// requests
pub mod req;

/// # The [`Step`](crate::step::Step) trait
/// The Step trait defines a powerful but simple API that allows
/// the CoAP runtime to be a composition of "steps," stored as a
/// type-level linked list.
///
/// e.g.
/// ```text
/// Gather Ingredients
///   -> Mix Wet Ingredients
///   -> Mix Dry Ingredients
///   -> Mix Everything together
///   -> Pour into cake tin
///   -> Bake
/// ```
/// as Steps:
/// ```text
/// Bake<PourIntoCakeTin<MixEverything<MixDry<MixWet<GatherIngredients<Empty>>>>>>
/// ```
///
/// ## Capabilities
///  * May read system state (time, dgram on the socket, platform configuration)
///     * [`platform::Snapshot`]
///  * May maintain internal state
///     * Must be managed with interior mutability (e.g. [`RwLock`](::std::sync::RwLock))
///  * May perform side effects
///     * [`platform::Effect`] provides deterministic API for logging and sending bytes over the wire
///  * May participate in client role, server role, or both roles in the CoAP runtime
///     * [`step::Step::poll_req`] (server)
///     * [`step::Step::poll_resp`] (client)
///  * May modify messages before they are sent
///     * [`step::Step::before_message_sent`]
///  * May be notified whenever a message is sent
///     * [`step::Step::on_message_sent`]
///  * May yield data to the outer step
///     * [`step::Step::PollReq`]
///     * [`step::Step::PollResp`]
///
/// ## Determinism
/// Steps provided by this crate will never perform any observable IO,
/// aside from managing their own internal state and appending to the list of
/// effects provided in the `poll_req`/`poll_resp` fns.
///
/// ## Logging
/// Steps provided by this crate will never log to any streams directly,
/// and will provide them via [`platform::Effect::Log`].
///
/// It is **strongly** recommended that [`log::Level::Warning`] and
/// [`log::Level::Error`] messages are not ignored.
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

/// TODO
pub mod server;

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
