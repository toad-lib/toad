//! Low-level representation of CoAP messages.
//!
//! Performs comparably&#42; to the `Packet` structure in [`coap_lite`](https://github.com/martindisch/coap-lite)
//! &#42; _(benchmark data available for [this library](./criterion/reports/kwap_msg_to_bytes/index.html) as well as [coap_lite](./criterion/reports/coap_lite_to_bytes/index.html))_
//!
//! If you're a library user, you probably want `req`/`resp` instead!

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

/// Identical to [`crate::no_alloc`], but replaces all fixed-capacity stack-allocated buffers with
/// dynamically growable structures.
///
/// Depends on crate feature `alloc` and `#[global_allocator]` (if your crate is `#[no_std]`).
#[cfg(feature = "alloc")]
#[cfg_attr(any(feature = "docs", docsrs), doc(cfg(feature = "alloc")))]
pub mod alloc;

/// Fixed capacity representations of raw CoAP messages.
pub mod no_alloc;

#[doc(hidden)]
pub mod parsing;

#[doc(inline)]
pub use parsing::TryFromBytes;

#[doc(hidden)]
pub mod to_bytes;

#[doc(inline)]
pub use to_bytes::TryIntoBytes;

#[doc(hidden)]
pub mod get_size;
#[doc(inline)]
pub use get_size::GetSize;
