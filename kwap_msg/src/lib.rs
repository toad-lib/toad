//! # kwap_msg
//! Low-level representation of CoAP messages.
//!
//! ## `alloc` vs `no_alloc`
//! kwap_msg implements CoAP messages as either backed by:
//! - `alloc`: dynamically growable heap-allocated buffers
//! - `no_alloc`: static stack-allocated buffers
//!
//! `alloc::Message` can be much easier to use and performs comparably to `no_alloc`, however it does require:
//! `std` or [a global allocator](https://doc.rust-lang.org/std/alloc/index.html)
//!
//! ## Performance
//! This crate uses `criterion` to measure performance of the heaped & heapless implementations in this crate as well as `coap_lite::Packet`.
//!
//! In general, `kwap_msg::alloc::Message` is faster than coap_lite, which is much faster than `no_alloc::Message`.
//!
//! Benchmarks:
//! ### Serializing to bytes
//! ![chart](https://raw.githubusercontent.com/clov-coffee/kwap/main/kwap_msg/docs/from_bytes.svg)
//!
//! ### Deserializing from bytes
//! ![chart](https://raw.githubusercontent.com/clov-coffee/kwap/main/kwap_msg/docs/to_bytes.svg)

#![doc(html_root_url = "https://docs.rs/kwap-msg/0.1.6")]
/* TODO: make user-facing `kwap` crate and put this there
 * # `kwap`
 *
 * `kwap` is a Rust CoAP implementation that aims to be:
 * - Platform-independent
 * - Extensible
 * - Approachable
 *
 * ## CoAP
 * CoAP is an application-level network protocol that copies the semantics of HTTP
 * to an environment conducive to **constrained** devices. (weak hardware, small battery capacity, etc.)
 *
 * This means that you can write and run two-way RESTful communication
 * between devices very similarly to the networking semantics you are
 * most likely very familiar with.
 *
 * ### Similarities to HTTP
 * CoAP has the same verbs and many of the same semantics as HTTP;
 * - GET, POST, PUT, DELETE
 * - Headers (renamed to [Options](https://datatracker.ietf.org/doc/html/rfc7252#section-5.10))
 * - Data format independent (via the [Content-Format](https://datatracker.ietf.org/doc/html/rfc7252#section-12.3) Option)
 * - [Response status codes](https://datatracker.ietf.org/doc/html/rfc7252#section-5.9)
 *
 * ### Differences from HTTP
 * - CoAP customarily sits on top of UDP (however the standard is [in the process of being adapted](https://tools.ietf.org/id/draft-ietf-core-coap-tcp-tls-11.html) to also run on TCP, like HTTP)
 * - Because UDP is a "connectionless" protocol, it offers no guarantee of "conversation" between traditional client and server roles. All the UDP transport layer gives you is a method to listen for messages thrown at you, and to throw messages at someone. Owing to this, CoAP machines are expected to perform both client and server roles (or more accurately, _sender_ and _receiver_ roles)
 * - While _classes_ of status codes are the same (Success 2xx -> 2.xx, Client error 4xx -> 4.xx, Server error 5xx -> 5.xx), the semantics of the individual response codes differ.
 */
#![cfg_attr(all(not(test), feature = "no_std"), no_std)]
#![cfg_attr(not(test), forbid(missing_debug_implementations, unreachable_pub))]
#![cfg_attr(not(test), deny(unsafe_code, missing_copy_implementations))]
#![cfg_attr(any(docsrs, feature = "docs"), feature(doc_cfg))]
#![deny(missing_docs)]

#[cfg(feature = "alloc")]
extern crate alloc as std_alloc;

/// Crate root for **allocating** CoAP messages
///
/// Depends on crate feature `alloc` and either `std` or a `#[global_allocator]`!
///
/// ```
/// use kwap_msg::alloc as msg;
/// use msg::{Message, TryFromBytes};
/// ```
#[cfg(feature = "alloc")]
#[cfg_attr(any(feature = "docs", docsrs), doc(cfg(feature = "alloc")))]
pub mod alloc;

/// Crate root for **non-allocating** CoAP messages
///
/// `no_alloc` is always available, even when crate feature `alloc` is enabled.
///
/// ```
/// use kwap_msg::no_alloc as msg;
/// use msg::{Message, TryFromBytes};
/// ```
pub mod no_alloc;

#[doc(hidden)]
pub mod from_bytes;
#[doc(hidden)]
pub mod get_size;
#[doc(hidden)]
pub mod to_bytes;

#[doc(inline)]
pub use from_bytes::{MessageParseError, OptParseError, TryFromBytes};
#[doc(inline)]
pub use get_size::GetSize;
#[doc(inline)]
pub use to_bytes::TryIntoBytes;

pub(crate) mod is_full;
