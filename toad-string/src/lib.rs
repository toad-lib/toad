//! This microcrate contains the stack-allocated `String` struct used by the
//! [`toad`](https://github.com/toad-lib/toad) CoAP runtime / ecosystem.

// docs
#![doc(html_root_url = "https://docs.rs/toad-string/0.0.0")]
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

use core::fmt::{Display, Write};
use core::ops::{Deref, DerefMut};

use tinyvec::ArrayVec;
use toad_array::AppendCopy;
use toad_len::Len;
use toad_writable::Writable;

#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash, Debug, Default)]
pub struct FromUtf8Error;

#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash, Debug, Default)]
pub struct FromUtf16Error;

/// [`String`]-returning copy of [`std::format`]
///
/// ```
/// use toad_string::{format, String};
/// assert_eq!(format!(32, "hello, {}!", String::<5>::from("jason")),
///            String::<32>::from("hello, jason!"));
/// ```
#[macro_export]
macro_rules! format {
  ($cap:literal, $($arg:tt)*) => {
    $crate::String::<$cap>::fmt(format_args!($($arg)*))
  };
}

/// Stack-allocated UTF-8 string with a fixed capacity.
///
/// Has many of the same inherent functions as [`std::string::String`].
#[derive(Debug, Copy, Clone, Default)]
pub struct String<const N: usize>(Writable<ArrayVec<[u8; N]>>);

impl<const N: usize> String<N> {
  /// Creates a new string with the specified capacity
  pub fn new() -> Self {
    Default::default()
  }

  /// Gets a string slice containing the entire [`String`]
  pub fn as_str(&self) -> &str {
    self.as_ref()
  }

  /// Convert the [`String`] to a mutable string slice
  pub fn as_mut_str(&mut self) -> &mut str {
    self.as_mut()
  }

  /// Resize the String to a new length
  ///
  /// If `M` is less than `N`, the extra bytes are
  /// discarded.
  pub fn resize<const M: usize>(&mut self) -> String<M> {
    let mut bytes = self.0.unwrap();
    bytes.truncate(M);
    String(Writable::from(self.as_writable().drain(..).collect::<ArrayVec<[u8; M]>>()))
  }

  /// Alias for [`AsRef`]
  pub fn as_bytes(&self) -> &[u8] {
    self.as_ref()
  }

  /// Get a mutable reference to the inner writable buffer
  pub fn as_writable(&mut self) -> &mut Writable<ArrayVec<[u8; N]>> {
    &mut self.0
  }

  /// Creates a [`String`] using the output of [`format_args`]
  pub fn fmt(args: core::fmt::Arguments) -> Self {
    let mut s = Self::new();
    s.write_fmt(args).ok();
    s
  }

  /// Returns this [`String`]'s capacity, in bytes.
  pub fn capacity(&self) -> usize {
    N
  }

  /// Truncates this [`String`], removing all contents.
  pub fn clear(&mut self) {
    self.0.clear()
  }

  /// Copy a slice of bytes to a `String`.
  ///
  /// A string ([`String`]) is made of bytes ([`u8`]), and a vector of bytes
  /// ([`Vec<u8>`]) is made of bytes, so this function converts between the
  /// two. Not all byte slices are valid `String`s, however: `String`
  /// requires that it is valid UTF-8. `from_utf8()` checks to ensure that
  /// the bytes are valid UTF-8, and then does the conversion.
  ///
  /// # Errors
  ///
  /// Returns [`Err`] if the slice is not UTF-8 with a description as to why the
  /// provided bytes are not UTF-8. The vector you moved in is also included.
  ///
  /// # Examples
  ///
  /// Basic usage:
  ///
  /// ```
  /// use toad_string::String;
  ///
  /// // some bytes, in a vector
  /// let sparkle_heart = vec![240, 159, 146, 150];
  ///
  /// // We know these bytes are valid, so we'll use `unwrap()`.
  /// let sparkle_heart = String::<16>::from_utf8(&sparkle_heart).unwrap();
  ///
  /// assert_eq!("💖", sparkle_heart);
  /// ```
  ///
  /// Incorrect bytes:
  ///
  /// ```
  /// use toad_string::String;
  ///
  /// // some invalid bytes, in a vector
  /// let sparkle_heart = vec![0, 159, 146, 150];
  ///
  /// assert!(String::<16>::from_utf8(&sparkle_heart).is_err());
  /// ```
  ///
  /// [`Vec<u8>`]: std::vec::Vec "Vec"
  /// [`&str`]: prim@str "&str"
  #[inline]
  pub fn from_utf8(bytes: &[u8]) -> Result<Self, FromUtf8Error> {
    match core::str::from_utf8(bytes) {
      | Ok(s) => Ok(Self::from(s)),
      | Err(_) => Err(FromUtf8Error),
    }
  }

  /// Decode a UTF-16–encoded vector `v` into a `String`, returning [`Err`]
  /// if `v` contains any invalid data.
  ///
  /// # Examples
  ///
  /// Basic usage:
  ///
  /// ```
  /// use toad_string::String;
  ///
  /// // 𝄞music
  /// let v = &[0xD834, 0xDD1E, 0x006d, 0x0075, 0x0073, 0x0069, 0x0063];
  /// assert_eq!(String::<16>::from("𝄞music"),
  ///            String::<16>::from_utf16(v).unwrap());
  ///
  /// // 𝄞mu<invalid>ic
  /// let v = &[0xD834, 0xDD1E, 0x006d, 0x0075, 0xD800, 0x0069, 0x0063];
  /// assert!(String::<16>::from_utf16(v).is_err());
  /// ```
  pub fn from_utf16(v: &[u16]) -> Result<Self, FromUtf16Error> {
    let mut ret = String::new();
    for c in char::decode_utf16(v.iter().cloned()) {
      if let Ok(c) = c {
        ret.push(c);
      } else {
        return Err(FromUtf16Error);
      }
    }
    Ok(ret)
  }

  /// Inserts a string slice into this `String` at a byte position.
  ///
  /// This is an *O*(*n*) operation as it requires copying every element in the
  /// buffer.
  ///
  /// # Panics
  ///
  /// Panics if `idx` is larger than the `String`'s length, or if it does not
  /// lie on a [`char`] boundary.
  ///
  /// # Examples
  ///
  /// Basic usage:
  ///
  /// ```
  /// use toad_string::String;
  ///
  /// let mut s = String::<16>::from("bar");
  ///
  /// s.insert_str(0, "foo");
  ///
  /// assert_eq!("foobar", s);
  /// ```
  #[inline]
  pub fn insert_str(&mut self, idx: usize, string: &str) {
    assert!(self.is_char_boundary(idx));

    for (i, b) in string.bytes().enumerate() {
      self.0.insert(idx + i, b);
    }
  }

  /// Inserts a character into this `String` at a byte position.
  ///
  /// This is an *O*(*n*) operation as it requires copying every element in the
  /// buffer.
  ///
  /// # Panics
  ///
  /// Panics if `idx` is larger than the `String`'s length, or if it does not
  /// lie on a [`char`] boundary.
  ///
  /// # Examples
  ///
  /// Basic usage:
  ///
  /// ```
  /// use toad_string::String;
  ///
  /// let mut s = String::<16>::new();
  ///
  /// s.insert(0, 'f');
  /// s.insert(1, 'o');
  /// s.insert(2, 'o');
  ///
  /// assert_eq!("foo", s);
  /// ```
  #[inline]
  pub fn insert(&mut self, idx: usize, ch: char) {
    assert!(self.is_char_boundary(idx));
    let mut bits = [0; 4];
    let bits = ch.encode_utf8(&mut bits).as_bytes();

    for (i, b) in bits.iter().enumerate() {
      self.0.insert(idx + i, *b);
    }
  }

  /// Appends the given [`char`] to the end of this `String`.
  ///
  /// # Examples
  ///
  /// Basic usage:
  ///
  /// ```
  /// use toad_string::String;
  ///
  /// let mut s = String::<16>::from("abc");
  ///
  /// s.push('1');
  /// s.push('2');
  /// s.push('3');
  ///
  /// assert_eq!("abc123", s);
  /// ```
  pub fn push(&mut self, ch: char) {
    match ch.len_utf8() {
      | 1 => self.0.push(ch as u8),
      | _ => self.0
                 .extend_from_slice(ch.encode_utf8(&mut [0; 4]).as_bytes()),
    }
  }

  /// Appends a given string slice onto the end of this `String`.
  ///
  /// # Examples
  ///
  /// Basic usage:
  ///
  /// ```
  /// use toad_string::String;
  ///
  /// let mut s = String::<16>::from("foo");
  ///
  /// s.push_str("bar");
  ///
  /// assert_eq!("foobar", s);
  /// ```
  #[inline]
  pub fn push_str(&mut self, string: &str) {
    self.0.append_copy(string.as_bytes())
  }
}

impl<const N: usize> Len for String<N> {
  const CAPACITY: Option<usize> = Some(N);

  fn len(&self) -> usize {
    self.0.len()
  }

  fn is_full(&self) -> bool {
    self.0.is_full()
  }
}

impl<const N: usize> Display for String<N> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "{}", self.as_str())
  }
}

impl<const N: usize> PartialEq for String<N> {
  fn eq(&self, other: &Self) -> bool {
    self.0.as_str() == other.0.as_str()
  }
}

impl<const N: usize> Eq for String<N> {}

impl<const N: usize> core::fmt::Write for String<N> {
  fn write_str(&mut self, s: &str) -> core::fmt::Result {
    self.0.write_str(s)
  }
}

impl<'a, const N: usize> From<&'a str> for String<N> {
  fn from(s: &'a str) -> Self {
    let mut arr = Writable::default();
    ArrayVec::extend_from_slice(&mut arr, s.as_bytes());

    Self(arr)
  }
}

impl<const N: usize> Deref for String<N> {
  type Target = str;
  fn deref(&self) -> &str {
    self.as_str()
  }
}

impl<const N: usize> DerefMut for String<N> {
  fn deref_mut(&mut self) -> &mut str {
    self.as_mut()
  }
}

impl<const N: usize> AsRef<str> for String<N> {
  fn as_ref(&self) -> &str {
    self.0.as_str()
  }
}

impl<const N: usize> AsMut<str> for String<N> {
  fn as_mut(&mut self) -> &mut str {
    core::str::from_utf8_mut(self.0.as_mut_slice()).unwrap()
  }
}

impl<const N: usize> AsRef<[u8]> for String<N> {
  fn as_ref(&self) -> &[u8] {
    self.0.as_str().as_bytes()
  }
}

impl<const N: usize> PartialEq<&str> for String<N> {
  fn eq(&self, other: &&str) -> bool {
    self.as_str() == *other
  }
}

impl<const N: usize> PartialEq<str> for String<N> {
  fn eq(&self, other: &str) -> bool {
    self.as_str() == other
  }
}

impl<const N: usize> PartialEq<String<N>> for &str {
  fn eq(&self, other: &String<N>) -> bool {
    *self == other.as_str()
  }
}

impl<const N: usize> PartialEq<&String<N>> for &str {
  fn eq(&self, other: &&String<N>) -> bool {
    *self == other.as_str()
  }
}
