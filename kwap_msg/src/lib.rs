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

/// A high-level encapsulation of a collection that can be exposed to (and extended by) users
///
/// This allows opt-in or -out of allocating data structures.
pub trait Collection<T>:
  Default + GetSize + Capacity + Extend<T> + FromIterator<T> + IntoIterator<Item = T>
    where for<'a> &'a Self: IntoIterator<Item = &'a T>
{
}

#[cfg(feature = "alloc")]
impl<T> Collection<T> for Vec<T> { }
impl<A: tinyvec::Array<Item = T>, T: 'static> Collection<T> for tinyvec::ArrayVec<A> { }

/*
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
*/

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

#[doc(hidden)]
pub mod is_full;
#[doc(inline)]
pub use is_full::{Capacity, IsFull};
#[cfg(feature = "alloc")]
use std_alloc::{vec::Vec, string::{String, ToString}};
use tinyvec::ArrayVec;

#[doc(hidden)]
pub mod opt;

#[doc(inline)]
pub use opt::*;

/// Low-level representation of the message payload
///
/// Both requests and responses may include a payload, depending on the
/// Method or Response Code, respectively.
///
/// # Related
/// - [RFC7252#section-5.5 Payloads and Representations](https://datatracker.ietf.org/doc/html/rfc7252#section-5.5)
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct Payload<C: Collection<u8>>(pub C) where for<'a> &'a C: IntoIterator<Item = &'a u8>;

///
#[cfg(feature = "alloc")]
pub type VecMessage = Message<Vec<u8>, Vec<u8>, Vec<Opt<Vec<u8>>>>;

///
pub type ArrayVecMessage<const PAYLOAD_CAP: usize, const N_OPTS: usize, const OPT_CAP: usize> = Message<ArrayVec<[u8; PAYLOAD_CAP]>, ArrayVec<[u8; OPT_CAP]>, ArrayVec<[Opt<ArrayVec<[u8; OPT_CAP]>>; N_OPTS]>>;

#[doc = include_str!("../docs/no_alloc/Message.md")]
#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub struct Message<PayloadC: Collection<u8>, OptC: Collection<u8> + 'static, Opts: Collection<Opt<OptC>>>
where for<'a> &'a PayloadC: IntoIterator<Item = &'a u8>,
    for<'a> &'a OptC: IntoIterator<Item = &'a u8>,
        for<'a> &'a Opts: IntoIterator<Item = &'a Opt<OptC>>{
  /// see [`Id`] for details
  pub id: Id,
  /// see [`Type`] for details
  pub ty: Type,
  /// see [`Version`] for details
  pub ver: Version,
  /// see [`Token`] for details
  pub token: Token,
  /// see [`Code`] for details
  pub code: Code,
  /// see [`opt::Opt`] for details
  pub opts: Opts,
  /// see [`Payload`]
  pub payload: Payload<PayloadC>,
  /// empty field using the Opt internal byte collection type
  pub __optc: core::marker::PhantomData<OptC>,
}

/// Struct representing the first byte of a message.
///
/// ```text
/// CoAP version
/// |
/// |  Message type (request, response, empty)
/// |  |
/// |  |  Length of token, in bytes. (4-bit integer)
/// |  |  |
/// vv vv vvvv
/// 01 00 0000
/// ```
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub(crate) struct Byte1 {
  pub(crate) ver: Version,
  pub(crate) ty: Type,
  pub(crate) tkl: u8,
}

/// Uniquely identifies a single message that may be retransmitted.
///
/// For a little more context and the difference between [`Id`] and [`Token`], see [`Token`].
///
/// See [RFC7252 - Message Details](https://datatracker.ietf.org/doc/html/rfc7252#section-3) for context
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct Id(pub u16);

/// Message type:
/// - 0 Confirmable; "Please let me know when you received this"
/// - 1 Non-confirmable; "I don't care if this gets to you"
/// - 2 Acknowledgement; "I got your message!"
/// - 3 Reset; ""
///
/// See [RFC7252 - Message Details](https://datatracker.ietf.org/doc/html/rfc7252#section-3) for context
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct Type(pub u8);

/// Version of the CoAP protocol that the message adheres to.
///
/// As far as this project is concerned, this will always be 1. (But will not _always_ be 1)
///
/// See [RFC7252 - Message Details](https://datatracker.ietf.org/doc/html/rfc7252#section-3) for context
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct Version(pub u8);

impl Default for Version {
  fn default() -> Self {
    Version(1)
  }
}

/// Low-level representation of the code of a message.
/// Identifying it as a request or response
///
/// # Examples
/// ```
/// use kwap_msg::Code;
/// assert_eq!(Code { class: 2, detail: 5 }.to_string(), "2.05".to_string())
/// ```
///
/// See [RFC7252 - Message Details](https://datatracker.ietf.org/doc/html/rfc7252#section-3) for context
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct Code {
  /// The "class" of message codes identify it as a request or response, and provides the class of response status:
  ///
  /// |class|meaning|
  /// |---|---|
  /// |`0`|Message is a request|
  /// |`2`|Message is a success response|
  /// |`4`|Message is a client error response|
  /// |`5`|Message is a server error response|
  pub class: u8,

  /// 2-digit integer (range `[0, 32)`) that provides granular information about the response status.
  ///
  /// Will always be `0` for requests.
  pub detail: u8,
}

impl Code {
  /// Get the human string representation of a message code
  ///
  /// # Returns
  /// A `char` array
  ///
  /// This is to avoid unnecessary heap allocation,
  /// you can create a `String` with `FromIterator::<String>::from_iter`,
  /// or if the `alloc` feature of `kwap` is enabled there is a `ToString` implementation provided for Code.
  /// ```
  /// use kwap_msg::Code;
  ///
  /// let code = Code { class: 2, detail: 5 };
  /// let chars = code.to_human();
  /// let string = String::from_iter(chars);
  /// assert_eq!(string, "2.05".to_string());
  /// ```
  pub fn to_human(&self) -> [char; 4] {
    let to_char = |d: u8| char::from_digit(d.into(), 10).unwrap();
    [to_char(self.class),
     '.',
     to_char(self.detail / 10),
     to_char(self.detail % 10)]
  }
}

/// Message token for matching requests to responses
///
/// Note that this is different from [`Id`],
/// which uniquely identifies a message that may be retransmitted.
///
/// For example, Client may send a confirmable message with id 1 and token 321
/// to Server multiple times,
/// then Server confirms and sends a response
/// with a different id (because it's a different message),
/// but token 321 (so the client knows which request the response is responding to)
///
/// Note that the format of the token is not necessarily an integer according to
/// the coap spec, but is interpreted by this library as an 8 byte unsigned integer in network byte order.
///
/// See [RFC7252 - Message Details](https://datatracker.ietf.org/doc/html/rfc7252#section-3) for context
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct Token(pub tinyvec::ArrayVec<[u8; 8]>);

impl ToString for Code {
  fn to_string(&self) -> String {
    String::from_iter(self.to_human())
  }
}

#[cfg(never)]
pub(self) fn test_msg() -> (Message, Vec<u8>) {
  let header: [u8; 4] = 0b01_00_0001_01000101_0000000000000001u32.to_be_bytes();
  let token: [u8; 1] = [254u8];
  let content_format: &[u8] = b"application/json";
  let options: [&[u8]; 2] = [&[0b_1100_1101u8, 0b00000011u8], content_format];
  let payload: [&[u8]; 2] = [&[0b_11111111u8], b"hello, world!"];
  let bytes = [header.as_ref(),
               token.as_ref(),
               options.concat().as_ref(),
               payload.concat().as_ref()].concat();

  let mut opts = Vec::new();
  let opt = Opt { delta: OptDelta(12),
                  value: OptValue(content_format.iter().copied().collect()) };
  opts.push(opt);

  let msg = Message { id: Id(1),
                      ty: Type(0),
                      ver: Version(1),
                      token: Token(tinyvec::array_vec!([u8; 8] => 254)),
                      opts,
                      code: Code { class: 2, detail: 5 },
                      payload: Payload(b"hello, world!".into_iter().copied().collect()) };
  (msg, bytes)
}
