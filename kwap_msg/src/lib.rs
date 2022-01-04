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
//! It solves this problem by being generic over the collections it needs and uses an `Array` trait
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
//! //                        |        |        Array of options in the message
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
//! <details><summary><b>Click to expand chart</b></summary>
//!
//! ![chart](https://raw.githubusercontent.com/clov-coffee/kwap/main/kwap_msg/docs/from_bytes.svg)
//! </details>
//!
//! ### Deserializing from bytes
//! <details><summary><b>Click to expand chart</b></summary>
//!
//! ![chart](https://raw.githubusercontent.com/clov-coffee/kwap/main/kwap_msg/docs/to_bytes.svg)
//! </details>

#![doc(html_root_url = "https://docs.rs/kwap-msg/0.3.0")]
#![cfg_attr(all(not(test), feature = "no_std"), no_std)]
#![cfg_attr(not(test), forbid(missing_debug_implementations, unreachable_pub))]
#![cfg_attr(not(test), deny(unsafe_code, missing_copy_implementations))]
#![cfg_attr(any(docsrs, feature = "docs"), feature(doc_cfg))]
#![deny(missing_docs)]

#[cfg(feature = "alloc")]
extern crate alloc as std_alloc;

#[doc(hidden)]
pub mod code;
#[doc(hidden)]
pub mod from_bytes;
#[doc(hidden)]
pub mod opt;
#[doc(hidden)]
pub mod to_bytes;

#[doc(inline)]
pub use code::*;
#[doc(inline)]
pub use from_bytes::{MessageParseError, OptParseError, TryFromBytes};
use kwap_common::{Array, GetSize};
use kwap_macros::rfc_7252_doc;
#[doc(inline)]
pub use opt::*;
#[cfg(feature = "alloc")]
use std_alloc::vec::Vec;
use tinyvec::ArrayVec;
#[doc(inline)]
pub use to_bytes::TryIntoBytes;

#[doc = rfc_7252_doc!("5.5")]
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct Payload<C: Array<Item = u8>>(pub C);

/// Message that uses Vec byte buffers
#[cfg(feature = "alloc")]
pub type VecMessage = Message<Vec<u8>, Vec<u8>, Vec<Opt<Vec<u8>>>>;

/// Message that uses static fixed-capacity stack-allocating byte buffers
pub type ArrayVecMessage<const PAYLOAD_CAP: usize, const N_OPTS: usize, const OPT_CAP: usize> =
  Message<ArrayVec<[u8; PAYLOAD_CAP]>, ArrayVec<[u8; OPT_CAP]>, ArrayVec<[Opt<ArrayVec<[u8; OPT_CAP]>>; N_OPTS]>>;

/// # `Message` struct
/// Low-level representation of a message that has been parsed from the raw binary format.
///
/// Note that `Message` is generic over 3 [`Array`]s:
///  - `PayloadC`: the byte buffer used to store the message's [`Payload`]
///  - `OptC`: byte buffer used to store [`Opt`]ion values ([`OptValue`])
///  - `Opts`: collection of [`Opt`]ions in the message
///
/// Messages support both serializing to bytes and from bytes, by using the provided [`TryFromBytes`] and [`TryIntoBytes`] traits.
///
/// <details>
/// <summary><b>RFC7252 - CoAP Messaging Model</b></summary>
#[doc = concat!("\n#", rfc_7252_doc!("2.1"))]
/// </details>
/// <details>
/// <summary><b>RFC7252 - CoAP Message Binary Format</b></summary>
#[doc = concat!("\n#", rfc_7252_doc!("3"))]
/// </details>
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
///   ty: Type::Con,
///   ver: Version(1),
///   token: Token(tinyvec::array_vec!([u8; 8] => 254)),
///   opts: opts_expected,
///   code: Code {class: 2, detail: 5},
///   payload: Payload(b"hello, world!".to_vec()),
/// };
///
/// assert_eq!(msg, expected);
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub struct Message<PayloadC: Array<Item = u8>, OptC: Array<Item = u8> + 'static, Opts: Array<Item = Opt<OptC>>> {
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
}

impl<P: Array<Item = u8>, O: Array<Item = u8>, Os: Array<Item = Opt<O>>> GetSize for Message<P, O, Os> {
  fn get_size(&self) -> usize {
    let header_size = 4;
    let payload_marker_size = 1;
    let payload_size = self.payload.0.get_size();
    let token_size = self.token.0.len();
    let opts_size: usize = self.opts.iter().map(|o| o.get_size()).sum();

    header_size + payload_marker_size + payload_size + token_size + opts_size
  }

  fn max_size(&self) -> Option<usize> {
    None
  }
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

/// # Message ID
///
/// 16-bit unsigned integer in network byte order.  Used to
/// detect message duplication and to match messages of type
/// Acknowledgement/Reset to messages of type Confirmable/Non-
/// confirmable.  The rules for generating a Message ID and matching
/// messages are defined in RFC7252 Section 4
///
/// For a little more context and the difference between [`Id`] and [`Token`], see [`Token`].
///
/// See [RFC7252 - Message Details](https://datatracker.ietf.org/doc/html/rfc7252#section-3) for context
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct Id(pub u16);

/// Indicates if this message is of
/// type Confirmable (0), Non-confirmable (1), Acknowledgement (2), or Reset (3).
///
/// See [RFC7252 - Message Details](https://datatracker.ietf.org/doc/html/rfc7252#section-3) for context
#[derive(Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Debug)]
pub enum Type {
  /// Some messages do not require an acknowledgement.  This is
  /// particularly true for messages that are repeated regularly for
  /// application requirements, such as repeated readings from a sensor.
  Non,
  /// Some messages require an acknowledgement.  These messages are
  /// called "Confirmable".  When no packets are lost, each Confirmable
  /// message elicits exactly one return message of type Acknowledgement
  /// or type Reset.
  Con,
  /// An Acknowledgement message acknowledges that a specific
  /// Confirmable message arrived.  By itself, an Acknowledgement
  /// message does not indicate success or failure of any request
  /// encapsulated in the Confirmable message, but the Acknowledgement
  /// message may also carry a Piggybacked Response.
  Ack,
  /// A Reset message indicates that a specific message (Confirmable or
  /// Non-confirmable) was received, but some context is missing to
  /// properly process it.  This condition is usually caused when the
  /// receiving node has rebooted and has forgotten some state that
  /// would be required to interpret the message.  Provoking a Reset
  /// message (e.g., by sending an Empty Confirmable message) is also
  /// useful as an inexpensive check of the liveness of an endpoint
  /// ("CoAP ping").
  Reset,
}

/// Version of the CoAP protocol that the message adheres to.
///
/// Right now, this will always be 1, but may support additional values in the future.
///
/// See [RFC7252 - Message Details](https://datatracker.ietf.org/doc/html/rfc7252#section-3) for context
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct Version(pub u8);

impl Default for Version {
  fn default() -> Self {
    Version(1)
  }
}
#[doc = rfc_7252_doc!("5.3.1")]
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct Token(pub tinyvec::ArrayVec<[u8; 8]>);

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
                         ty: Type::Con,
                         ver: Version(1),
                         token: Token(tinyvec::array_vec!([u8; 8] => 254)),
                         opts,
                         code: Code { class: 2, detail: 5 },
                         payload: Payload(b"hello, world!".into_iter().copied().collect()) };
  (msg, bytes)
}

#[cfg(test)]
pub(crate) mod tests {
  #[macro_export]
  macro_rules! assert_eqb {
    ($actual:expr, $expected:expr) => {
      if $actual != $expected {
        panic!("expected {:08b} to equal {:08b}", $actual, $expected)
      }
    };
  }

  #[macro_export]
  macro_rules! assert_eqb_iter {
    ($actual:expr, $expected:expr) => {
      if $actual.iter().ne($expected.iter()) {
        panic!("expected {:?} to equal {:?}",
               $actual.into_iter().map(|b| format!("{:08b}", b)).collect::<Vec<_>>(),
               $expected.into_iter().map(|b| format!("{:08b}", b)).collect::<Vec<_>>())
      }
    };
  }
}
