//! Common structs and abstractions used by `toad`

#![doc(html_root_url = "https://docs.rs/toad-common/0.11.1")]
#![cfg_attr(all(not(test), feature = "no_std"), no_std)]
#![cfg_attr(not(test), forbid(missing_debug_implementations, unreachable_pub))]
#![cfg_attr(not(test), deny(unsafe_code, missing_copy_implementations))]
#![allow(clippy::unused_unit)]
#![deny(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc as std_alloc;

/// Extensions to Result
pub mod result;
pub use result::*;

/// Function utils
pub mod fns;
pub use fns::*;

/// Cursor
pub mod cursor;
pub use cursor::*;

/// Map
pub mod map;
pub use map::*;

/// Array
pub mod array;
pub use array::*;

/// Writable
pub mod writable;
pub use writable::*;
