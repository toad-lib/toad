//! Common structs and abstractions used by `toad`

#![doc(html_root_url = "https://docs.rs/toad-common/0.10.0")]
#![cfg_attr(all(not(test), feature = "no_std"), no_std)]
#![cfg_attr(not(test), forbid(missing_debug_implementations, unreachable_pub))]
#![cfg_attr(not(test), deny(unsafe_code, missing_copy_implementations))]
#![allow(clippy::unused_unit)]
#![deny(missing_docs)]

extern crate alloc;

/// Extensions to Result
pub mod result;

/// Function utils
pub mod fns;

/// Cursor
pub mod cursor;
pub use cursor::*;

/// Map
pub mod map;
pub use map::*;

/// Array
pub mod array;
pub use array::*;

/// `toad` prelude
pub mod prelude {
  pub use array::*;
  pub use cursor::*;
  pub use fns::*;
  pub use map::*;
  pub use result::*;

  pub use super::*;
}
