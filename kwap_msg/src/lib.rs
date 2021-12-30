//! # kwap_msg
//! Low-level representation of CoAP messages.
//!
//! The most notable item in `kwap_msg` is `Message`;
//! a CoAP message very close to the actual byte layout.
//!
//! ## Allocation
//! CoAP messages have some attributes whose size is dynamic:
//! - The message payload (in http terms: the request/response body)
//! - the number of options (in http terms: headers)
//! - the value of an option (in http terms: header value)
//!
//! `Message` does not require an allocator and has no opinions about what kind of collection
//! it uses internally to store these values.
//!
//! It solves this problem by being generic over the collections it needs and uses a `Collection` trait
//! to capture its idea of what makes a collection useful.
//!
//! This means that you may use a provided implementation (for `Vec` or `tinyvec::ArrayVec`)
//! or provide your own collection (see the [custom collections example](https://github.com/clov-coffee/kwap/blob/main/kwap_msg/examples/custom_collections.rs))
//!
//! ```rust
//! //! Note: both of these type aliases are exported by `kwap_msg` for convenience.
//!
//! use tinyvec::ArrayVec;
//! use kwap_msg::{Message, Opt};
//!
//! //                        Message Payload byte buffer
//! //                        |
//! //                        |        Option Value byte buffer
//! //                        |        |
//! //                        |        |        Collection of options in the message
//! //                        vvvvvvv  vvvvvvv  vvvvvvvvvvvvvvvvv
//! type VecMessage = Message<Vec<u8>, Vec<u8>, Vec<Opt<Vec<u8>>>>;
//!
//! // Used like: `ArrayVecMessage<1024, 256, 16>`; a message that can store a payload up to 1024 bytes, and up to 16 options each with up to a 256 byte value.
//! type ArrayVecMessage<
//!        const PAYLOAD_SIZE: usize,
//!        const OPT_SIZE: usize,
//!        const NUM_OPTS: usize,
//!      > = Message<
//!            ArrayVec<[u8; PAYLOAD_SIZE]>,
//!            ArrayVec<[u8; OPT_SIZE]>,
//!            ArrayVec<[Opt<ArrayVec<[u8; OPT_SIZE]>>; NUM_OPTS]>,
//!          >;
//! ```
//!
//! It may look a little ugly, but a core goal of `kwap` is to be platform- and alloc-agnostic.
//!
//! ## Performance
//! This crate uses `criterion` to measure performance of the heaped & heapless implementations in this crate as well as `coap_lite::Packet`.
//!
//! In general, `kwap_msg::VecMessage` performs identically to coap_lite (+/- 5%), and both are **much** faster than `kwap_msg::ArrayVecMessage`.
//!
//! Benchmarks:
//! ### Serializing to bytes
//! ![chart](https://raw.githubusercontent.com/clov-coffee/kwap/main/kwap_msg/docs/from_bytes.svg)
//!
//! ### Deserializing from bytes
//! ![chart](https://raw.githubusercontent.com/clov-coffee/kwap/main/kwap_msg/docs/to_bytes.svg)

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
#![doc(html_root_url = "https://docs.rs/kwap-msg/0.2.2")]
#![cfg_attr(all(not(test), feature = "no_std"), no_std)]
#![cfg_attr(not(test), forbid(missing_debug_implementations, unreachable_pub))]
#![cfg_attr(not(test), deny(unsafe_code, missing_copy_implementations))]
#![cfg_attr(any(docsrs, feature = "docs"), feature(doc_cfg))]
#![deny(missing_docs)]

#[cfg(feature = "alloc")]
extern crate alloc as std_alloc;
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
pub use is_full::Reserve;
#[cfg(feature = "alloc")]
use std_alloc::{string::{String, ToString},
                vec::Vec};
use tinyvec::ArrayVec;

#[doc(hidden)]
pub mod opt;

#[doc(inline)]
pub use opt::*;

/// Any collection may be used to store bytes in CoAP Messages :)
///
/// # Provided implementations
/// - [`Vec`]
/// - [`tinyvec::ArrayVec`]
///
/// Notably, not `heapless::ArrayVec` or `arrayvec::ArrayVec`. An important usecase
/// is [`Extend`]ing the collection, and the performance of `heapless` and `arrayvec`'s Extend implementations
/// are notably worse than `tinyvec`.
///
/// `tinyvec` also has the added bonus of being 100% unsafe-code-free, meaning if you choose `tinyvec` you eliminate the
/// possibility of memory defects and UB.
///
/// # Requirements
/// - `Default` for creating the collection
/// - `Extend` for mutating and adding onto the collection (1 or more elements)
/// - `Reserve` for reserving space ahead of time
/// - `GetSize` for bound checks, empty checks, and accessing the length
/// - `FromIterator` for collecting into the collection
/// - `IntoIterator` for:
///    - iterating and destroying the collection
///    - for iterating over references to items in the collection
///
/// # Stupid `where` clause
/// `where for<'a> &'a Self: IntoIterator<Item = &'a T>` is necessary to fold in the idea
/// of "A reference (of any arbitrary lifetime `'a`) to a Collection must support iterating over references (`'a`) of its elements."
///
/// A side-effect of this where clause is that because it's not a trait bound, it must be propagated to every bound that requires a `Collection`.
///
/// Less than ideal, but far preferable to coupling tightly to a particular collection and maintaining separate `alloc` and non-`alloc` implementations.
pub trait Collection<T>: Default + GetSize + Reserve + Extend<T> + FromIterator<T> + IntoIterator<Item = T>
  where for<'a> &'a Self: IntoIterator<Item = &'a T>
{
}

#[cfg(feature = "alloc")]
impl<T> Collection<T> for Vec<T> {}
impl<A: tinyvec::Array<Item = T>, T> Collection<T> for tinyvec::ArrayVec<A> {}

/// Low-level representation of the message payload
///
/// Both requests and responses may include a payload, depending on the
/// Method or Response Code, respectively.
///
/// # Related
/// - [RFC7252#section-5.5 Payloads and Representations](https://datatracker.ietf.org/doc/html/rfc7252#section-5.5)
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct Payload<C: Collection<u8>>(pub C) where for<'a> &'a C: IntoIterator<Item = &'a u8>;

/// Message that uses Vec byte buffers
#[cfg(feature = "alloc")]
pub type VecMessage = Message<Vec<u8>, Vec<u8>, Vec<Opt<Vec<u8>>>>;

/// Message that uses static fixed-capacity stack-allocating byte buffers
pub type ArrayVecMessage<const PAYLOAD_CAP: usize, const N_OPTS: usize, const OPT_CAP: usize> =
  Message<ArrayVec<[u8; PAYLOAD_CAP]>, ArrayVec<[u8; OPT_CAP]>, ArrayVec<[Opt<ArrayVec<[u8; OPT_CAP]>>; N_OPTS]>>;

/// Low-level representation of a message
/// that has been parsed from a byte array
///
/// To convert an iterator of bytes into a Message, there is a provided trait [`crate::TryFromBytes`].
///
/// ```
/// use kwap_msg::TryFromBytes;
/// use kwap_msg::*;
/// # //                       version  token len  code (2.05 Content)
/// # //                       |        |          /
/// # //                       |  type  |         /  message ID
/// # //                       |  |     |        |   |
/// # //                       vv vv vvvv vvvvvvvv vvvvvvvvvvvvvvvv
/// # let header: [u8; 4] = 0b_01_00_0001_01000101_0000000000000001u32.to_be_bytes();
/// # let token: [u8; 1] = [254u8];
/// # let content_format: &[u8] = b"application/json";
/// # let options: [&[u8]; 2] = [&[0b_1100_1101u8, 0b00000011u8], content_format];
/// # let payload: [&[u8]; 2] = [&[0b_11111111u8], b"hello, world!"];
/// let packet: Vec<u8> = /* bytes! */
/// # [header.as_ref(), token.as_ref(), options.concat().as_ref(), payload.concat().as_ref()].concat();
///
/// // `VecMessage` uses `Vec` as the backing structure for byte buffers
/// let msg = VecMessage::try_from_bytes(packet.clone()).unwrap();
/// # let opt = Opt {
/// #   delta: OptDelta(12),
/// #   value: OptValue(content_format.iter().map(|u| *u).collect()),
/// # };
/// let mut opts_expected = /* create expected options */
/// # Vec::new();
/// # opts_expected.push(opt);
///
/// let expected = VecMessage {
///   id: Id(1),
///   ty: Type(0),
///   ver: Version(1),
///   token: Token(tinyvec::array_vec!([u8; 8] => 254)),
///   opts: opts_expected,
///   code: Code {class: 2, detail: 5},
///   payload: Payload(b"hello, world!".to_vec()),
///   __optc: Default::default(),
/// };
///
/// assert_eq!(msg, expected);
/// ```
///
/// See [RFC7252 - Message Details](https://datatracker.ietf.org/doc/html/rfc7252#section-3) for context
#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub struct Message<PayloadC: Collection<u8>, OptC: Collection<u8> + 'static, Opts: Collection<Opt<OptC>>>
  where for<'a> &'a PayloadC: IntoIterator<Item = &'a u8>,
        for<'a> &'a OptC: IntoIterator<Item = &'a u8>,
        for<'a> &'a Opts: IntoIterator<Item = &'a Opt<OptC>>
{
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

// NOTE: duplicated in tests/common
#[cfg(test)]
pub(crate) fn test_msg() -> (VecMessage, Vec<u8>) {
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

  let msg = VecMessage { id: Id(1),
                         ty: Type(0),
                         ver: Version(1),
                         token: Token(tinyvec::array_vec!([u8; 8] => 254)),
                         opts,
                         code: Code { class: 2, detail: 5 },
                         payload: Payload(b"hello, world!".into_iter().copied().collect()),
                         __optc: Default::default() };
  (msg, bytes)
}
