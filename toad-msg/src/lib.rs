//! Low-level representation of CoAP messages.
//!
//! The most notable item in `toad_msg` is `Message`;
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
//! or provide your own collection (see the [custom collections example](https://github.com/clov-coffee/toad/blob/main/toad_msg/examples/custom_collections.rs))
//!
//! ```rust
//! //! Note: both of these type aliases are exported by `toad_msg` for convenience.
//!
//! use tinyvec::ArrayVec;
//! use toad_msg::{Message, Opt};
//!
//! //                        Message Payload byte buffer
//! //                        |
//! //                        |        Array of options in the message
//! //                        vvvvvvv  vvvvvvvvvvvvvvvvv
//! type VecMessage = Message<Vec<u8>, Vec<Opt<Vec<u8>>>>;
//!
//! // Used like: `ArrayVecMessage<1024, 256, 16>`; a message that can store a payload up to 1024 bytes, and up to 16 options each with up to a 256 byte value.
//! type ArrayVecMessage<
//!        const PAYLOAD_SIZE: usize,
//!        const OPT_SIZE: usize,
//!        const NUM_OPTS: usize,
//!      > = Message<
//!            ArrayVec<[u8; PAYLOAD_SIZE]>,
//!            ArrayVec<[Opt<ArrayVec<[u8; OPT_SIZE]>>; NUM_OPTS]>,
//!          >;
//! ```
//!
//! It may look a little ugly, but a core goal of `toad` is to be platform- and alloc-agnostic.
//!
//! ## Performance
//! This crate uses `criterion` to measure performance of the heaped & heapless implementations in this crate as well as `coap_lite::Packet`.
//!
//! In general, `toad_msg::VecMessage` performs identically to coap_lite (+/- 5%), and both are **much** faster than `toad_msg::ArrayVecMessage`.
//!
//! Benchmarks:
//! ### Serializing to bytes
//! <details>
//! <summary>
//!
//! **Click to expand chart**
//! </summary>
//!
//! ![chart](https://raw.githubusercontent.com/clov-coffee/toad/main/toad-msg/docs/from_bytes.svg)
//! </details>
//!
//! ### Deserializing from bytes
//! <details>
//! <summary>
//!
//! **Click to expand chart**
//! </summary>
//!
//! ![chart](https://raw.githubusercontent.com/clov-coffee/toad/main/toad-msg/docs/to_bytes.svg)
//! </details>

// x-release-please-start-version
#![doc(html_root_url = "https://docs.rs/toad-msg/0.12.1")]
// x-release-please-end
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(test), forbid(missing_debug_implementations, unreachable_pub))]
#![cfg_attr(not(test), deny(unsafe_code, missing_copy_implementations))]
#![cfg_attr(any(docsrs, feature = "docs"), feature(doc_cfg))]
#![deny(missing_docs)]

#[cfg(feature = "alloc")]
extern crate alloc as std_alloc;

#[doc(hidden)]
pub mod from_bytes;

/// Message structs
pub mod msg;

#[doc(hidden)]
pub mod to_bytes;

#[doc(inline)]
pub use from_bytes::TryFromBytes;
#[doc(inline)]
pub use msg::*;
#[doc(inline)]
pub use to_bytes::TryIntoBytes;
use toad_common::Array;

/// Type aliases for std or alloc platforms
#[cfg(feature = "alloc")]
pub mod alloc {
  use std_alloc::collections::BTreeMap;
  use std_alloc::vec::Vec;

  use crate::{OptNumber, OptValue};

  /// [`crate::Message`] that uses Vec and BTreeMap
  pub type Message = crate::Message<Vec<u8>, BTreeMap<OptNumber, Vec<OptValue<Vec<u8>>>>>;
}

#[cfg(test)]
pub(crate) fn test_msg() -> (alloc::Message, Vec<u8>) {
  use std_alloc::collections::BTreeMap;
  // TEST

  let header: [u8; 4] = 0b0100_0001_0100_0101_0000_0000_0000_0001_u32.to_be_bytes();
  let token: [u8; 1] = [254u8];
  let content_format: &[u8] = b"application/json";
  let options: [&[u8]; 2] = [&[0b_1100_1101u8, 0b00000011u8], content_format];
  let payload: [&[u8]; 2] = [&[0b1111_1111_u8], b"hello, world!"];
  let bytes = [header.as_ref(),
               token.as_ref(),
               options.concat().as_ref(),
               payload.concat().as_ref()].concat();

  let msg = alloc::Message { id: Id(1),
                             ty: Type::Con,
                             ver: Version(1),
                             token: Token(tinyvec::array_vec!([u8; 8] => 254)),
                             opts: BTreeMap::from([(OptNumber(12),
                                                    vec![OptValue(content_format.to_vec())])]),
                             code: Code { class: 2,
                                          detail: 5 },
                             payload: Payload(b"hello, world!".to_vec()) };
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
               $actual.into_iter()
                      .map(|b| format!("{:08b}", b))
                      .collect::<Vec<_>>(),
               $expected.into_iter()
                        .map(|b| format!("{:08b}", b))
                        .collect::<Vec<_>>())
      }
    };
  }
}
