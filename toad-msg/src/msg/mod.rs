use core::cmp::Ordering;
use core::hash::Hash;
use core::iter::FromIterator;
use core::str::{from_utf8, Utf8Error};

use toad_array::{AppendCopy, Array};
use toad_cursor::Cursor;
use toad_len::Len;
use toad_macros::rfc_7252_doc;

#[allow(unused_imports)]
use crate::TryIntoBytes;

/// Message Code
pub mod code;

/// Message parsing errors
pub mod parse_error;

/// Message ID
pub mod id;

/// Message Options
pub mod opt;

/// Message Type
pub mod ty;

/// Message Token
pub mod token;

/// Message Version
pub mod ver;

pub use code::*;
pub use id::*;
pub use opt::*;
pub use parse_error::*;
pub use token::*;
pub use ty::*;
pub use ver::*;

use crate::from_bytes::TryConsumeBytes;
use crate::TryFromBytes;

#[doc = rfc_7252_doc!("5.5")]
#[derive(Default, Clone, Debug)]
pub struct Payload<C>(pub C);

impl<C> PartialOrd for Payload<C> where C: Array<Item = u8>
{
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    self.0.iter().partial_cmp(other.0.iter())
  }
}

impl<C> PartialEq for Payload<C> where C: Array<Item = u8>
{
  fn eq(&self, other: &Self) -> bool {
    self.0.iter().eq(other.0.iter())
  }
}

impl<C> Ord for Payload<C> where C: Array<Item = u8>
{
  fn cmp(&self, other: &Self) -> Ordering {
    self.0.iter().cmp(other.0.iter())
  }
}

impl<C> Eq for Payload<C> where C: Array<Item = u8> {}

impl<C> Hash for Payload<C> where C: Array<Item = u8>
{
  fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
    state.write(&self.0)
  }
}

impl<C> Payload<C> where C: Array<Item = u8>
{
  /// Convert a reference to a Payload to a byte slice
  pub fn as_bytes(&self) -> &[u8] {
    &self.0
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
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub(crate) struct Byte1 {
  pub(crate) ver: Version,
  pub(crate) ty: Type,
  pub(crate) tkl: u8,
}

impl TryFrom<u8> for Byte1 {
  type Error = MessageParseError;

  fn try_from(b: u8) -> Result<Self, Self::Error> {
    let ver = b >> 6; // bits 0 & 1
    let ty = b >> 4 & 0b11; // bits 2 & 3
    let tkl = b & 0b1111u8; // last 4 bits

    Ok(Byte1 { ver: Version(ver),
               ty: Type::try_from(ty)?,
               tkl })
  }
}

impl<PayloadBytes: Array<Item = u8>, Options: OptionMap> Len for Message<PayloadBytes, Options> {
  const CAPACITY: Option<usize> = None;

  fn len(&self) -> usize {
    let header_size = 4;
    let payload_marker_size = 1;
    let payload_size = self.payload.0.len();
    let token_size = self.token.0.len();
    let opts_size: usize = self.opts.opt_refs().map(|o| o.len()).sum();

    header_size + payload_marker_size + payload_size + token_size + opts_size
  }

  fn is_full(&self) -> bool {
    false
  }
}

/// # CoAP Messages
/// This struct provides a high-level API for manipulating requests & responses,
/// while still being cheaply serializable to & from the byte layout of CoAP messages on the wire.
///
/// ## Options
/// Options (the CoAP equivalent to HTTP headers) can be manipulated with methods
/// provided in the [`MessageOptions`] trait. This includes getting & setting common
/// options known to this library.
///
/// ## Constructing
/// [`Message::new`] is the most straightforward way to initialize messages.
///
/// Being one of the few structs in the toad-lib libraries with public fields,
/// you may also initialize it with a struct literal.
///
/// ```
/// use toad_msg::alloc::Message;
/// use toad_msg::{Code, Id, Payload, Token, Type, Version};
///
/// let a = Message { id: Id(1),
///                   token: Token(Default::default()),
///                   ver: Version::default(),
///                   ty: Type::Con,
///                   code: Code::GET,
///                   payload: Payload(vec![]),
///                   opts: Default::default() };
///
/// let b = Message::new(Type::Con, Code::GET, Id(1), Token(Default::default()));
///
/// assert_eq!(a, b);
/// ```
///
/// ## Sending / Receiving
/// This crate (`toad-msg`) explicitly does **not** know or care about how
/// the messages are sent and received, and is **just** concerned with the data
/// structures involved on the machines having a CoAP conversation.
///
/// For a runtime that uses this library, see [`toad`](https://www.docs.rs/toad/latest).
///
/// <details>
/// <summary><b>Further Reading from RFC7252</b></summary>
#[doc = concat!("\n\n#", rfc_7252_doc!("2.1"))]
#[doc = concat!("\n\n#", rfc_7252_doc!("3"))]
/// </details>
#[derive(Clone, Debug)]
pub struct Message<PayloadBytes, Options> {
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
  pub opts: Options,
  /// see [`Payload`]
  pub payload: Payload<PayloadBytes>,
}

impl<C, O> PartialOrd for Message<C, O>
  where O: OptionMap + PartialOrd,
        C: Array<Item = u8>
{
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}
impl<C, O> PartialEq for Message<C, O>
  where O: OptionMap + PartialEq,
        C: Array<Item = u8>
{
  fn eq(&self, other: &Self) -> bool {
    self.id == other.id
    && self.ver == other.ver
    && self.code == other.code
    && self.token == other.token
    && self.payload == other.payload
    && self.opts == other.opts
  }
}
impl<C, O> Ord for Message<C, O>
  where O: OptionMap + PartialOrd,
        C: Array<Item = u8>
{
  fn cmp(&self, other: &Self) -> Ordering {
    self.id
        .cmp(&other.id)
        .then(self.ver.cmp(&other.ver))
        .then(self.code.cmp(&other.code))
        .then(self.token.cmp(&other.token))
        .then(self.payload.cmp(&other.payload))
        .then(self.opts
                  .partial_cmp(&other.opts)
                  .unwrap_or(Ordering::Equal))
  }
}
impl<C, O> Eq for Message<C, O>
  where O: OptionMap + PartialEq,
        C: Array<Item = u8>
{
}

impl<C, O> Hash for Message<C, O>
  where O: OptionMap + PartialEq + Hash,
        C: Array<Item = u8>
{
  fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
    self.id.hash(state);
    self.code.hash(state);
    self.token.hash(state);
    self.ver.hash(state);
    self.ty.hash(state);
    self.opts.hash(state);
    self.payload.hash(state);
  }
}

/// An error occurred during a call to [`Message::set`]
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SetOptionError<OV, OVs> {
  RepeatedTooManyTimes(OV),
  TooManyOptions(OptNumber, OVs),
}

impl<PayloadBytes: Array<Item = u8> + AppendCopy<u8>, Options: OptionMap> MessageOptions
  for Message<PayloadBytes, Options>
{
  type OptValues = Options::OptValues;
  type OptValueBytes = Options::OptValue;
  type SetError = SetOptionError<OptValue<Self::OptValueBytes>, Self::OptValues>;

  fn add(&mut self, n: OptNumber, v: OptValue<Self::OptValueBytes>) -> Result<(), Self::SetError> {
    self.add(n, v)
  }

  fn set(&mut self,
         n: OptNumber,
         v: OptValue<Self::OptValueBytes>)
         -> Result<Option<Self::OptValues>, Self::SetError> {
    self.set(n, v)
  }

  fn count(&self, n: OptNumber) -> usize {
    self.count(n)
  }

  fn get(&self, n: OptNumber) -> Option<&Self::OptValues> {
    self.get(n)
  }

  fn get_first(&self, n: OptNumber) -> Option<&OptValue<Self::OptValueBytes>> {
    self.get_first(n)
  }

  fn get_str(&self, n: OptNumber) -> Result<Option<&str>, Utf8Error> {
    self.get_str(n)
  }

  fn get_strs<'a, F>(&'a self, n: OptNumber) -> Result<F, Utf8Error>
    where F: FromIterator<&'a str>
  {
    self.get_strs(n)
  }

  fn get_u8(&self, n: OptNumber) -> Option<u8> {
    self.get_u8(n)
  }

  fn get_u16(&self, n: OptNumber) -> Option<u16> {
    self.get_u16(n)
  }

  fn get_u32(&self, n: OptNumber) -> Option<u32> {
    self.get_u32(n)
  }

  fn get_u64(&self, n: OptNumber) -> Option<u64> {
    self.get_u64(n)
  }

  fn remove(&mut self, n: OptNumber) -> Option<Self::OptValues> {
    self.remove(n)
  }
}

/// Methods that allow accessing & setting options known to the toad library.
pub trait MessageOptions {
  /// [`OptionMap::OptValues`]
  type OptValues: Array<Item = OptValue<Self::OptValueBytes>>;
  /// [`OptionMap::OptValue`]
  type OptValueBytes: Array<Item = u8> + AppendCopy<u8>;
  /// [`SetOptionError`]
  type SetError;

  /// Insert a new value for a given option
  ///
  /// Errors when there cannot be any more options, or the option
  /// cannot be repeated any more (only applies to non-std environments)
  #[doc = rfc_7252_doc!("5.4.5")]
  fn add(&mut self, n: OptNumber, v: OptValue<Self::OptValueBytes>) -> Result<(), Self::SetError>;

  /// Replace any / all existing values with a new one,
  /// yielding the previous value(s)
  fn set(&mut self,
         n: OptNumber,
         v: OptValue<Self::OptValueBytes>)
         -> Result<Option<Self::OptValues>, Self::SetError>;

  /// Get the number of values for a given option
  fn count(&self, n: OptNumber) -> usize;

  /// Get the value(s) of an option by number
  ///
  /// This just invokes [`toad_common::Map::get`] on [`Message.opts`].
  fn get(&self, n: OptNumber) -> Option<&Self::OptValues>;

  /// Get the value of an option, taking the first if there are multiple.
  fn get_first(&self, n: OptNumber) -> Option<&OptValue<Self::OptValueBytes>>;

  /// Get the value of an option, and interpret it
  /// as a UTF-8 string
  fn get_str(&self, n: OptNumber) -> Result<Option<&str>, Utf8Error>;

  /// Get all values for an option, and interpret them as UTF-8 strings
  fn get_strs<'a, F>(&'a self, n: OptNumber) -> Result<F, Utf8Error>
    where F: FromIterator<&'a str>;

  /// Get the value of an option, and interpret it
  /// as a u8
  fn get_u8(&self, n: OptNumber) -> Option<u8>;

  /// Get the value of an option, and interpret it
  /// as a u16
  fn get_u16(&self, n: OptNumber) -> Option<u16>;

  /// Get the value of an option, and interpret it
  /// as a u32
  fn get_u32(&self, n: OptNumber) -> Option<u32>;

  /// Get the value of an option, and interpret it
  /// as a u64
  fn get_u64(&self, n: OptNumber) -> Option<u64>;

  /// Remove all values for the option from this message,
  /// returning them if there were any.
  fn remove(&mut self, n: OptNumber) -> Option<Self::OptValues>;

  /// Update the value for the [Uri-Host](opt::known::no_repeat::HOST) option,
  /// discarding any existing values.
  ///
  /// ```
  /// use toad_msg::alloc::Message;
  /// use toad_msg::{Code, Id, MessageOptions, Token, Type};
  ///
  /// let mut msg = Message::new(Type::Con, Code::GET, Id(1), Token(Default::default()));
  ///
  /// msg.set_host("cheese.com").unwrap();
  /// assert_eq!(msg.host(), Ok(Some("cheese.com")));
  /// ```
  #[doc = rfc_7252_doc!("5.10.1")]
  fn set_host<S>(&mut self, host: S) -> Result<(), Self::SetError>
    where S: AsRef<str>
  {
    self.set(opt::known::no_repeat::HOST,
             host.as_ref().as_bytes().iter().copied().collect())
        .map(|_| ())
  }

  /// [`opt::known::no_repeat::BLOCK1`]
  fn block1(&self) -> Option<block::Block> {
    self.get_u32(opt::known::no_repeat::BLOCK1)
        .map(block::Block::from)
  }

  /// [`opt::known::no_repeat::BLOCK1`]
  fn set_block1(&mut self, size: u16, num: u32, more: bool) -> Result<(), Self::SetError> {
    let block = block::Block::new(size, num, more);
    self.set(opt::known::no_repeat::BLOCK1,
             OptValue(u32::from(block).to_be_bytes().iter().copied().collect()))
        .map(|_| ())
  }

  /// [`opt::known::no_repeat::BLOCK2`]
  fn block2(&self) -> Option<block::Block> {
    self.get_u32(opt::known::no_repeat::BLOCK2)
        .map(block::Block::from)
  }

  /// [`opt::known::no_repeat::BLOCK2`]
  fn set_block2(&mut self, size: u16, num: u32, more: bool) -> Result<(), Self::SetError> {
    let block = block::Block::new(size, num, more);
    self.set(opt::known::no_repeat::BLOCK2,
             OptValue(u32::from(block).to_be_bytes().iter().copied().collect()))
        .map(|_| ())
  }

  /// Get the value for the [Uri-Host](opt::known::no_repeat::HOST) option
  fn host(&self) -> Result<Option<&str>, Utf8Error> {
    self.get_str(opt::known::no_repeat::HOST)
  }

  /// Update the value for the [Uri-Port](opt::known::no_repeat::PORT) option,
  /// discarding any existing values.
  ///
  /// ```
  /// use toad_msg::alloc::Message;
  /// use toad_msg::{Code, Id, MessageOptions, Token, Type};
  ///
  /// let mut msg = Message::new(Type::Con, Code::GET, Id(1), Token(Default::default()));
  ///
  /// msg.set_host("cheese.com").unwrap();
  /// msg.set_port(1234).unwrap();
  /// assert_eq!(msg.host(), Ok(Some("cheese.com")));
  /// assert_eq!(msg.port(), Some(1234));
  /// ```
  fn set_port(&mut self, port: u16) -> Result<(), Self::SetError> {
    self.set(opt::known::no_repeat::PORT,
             port.to_be_bytes().into_iter().collect())
        .map(|_| ())
  }

  /// Get the value for the [Uri-Port](opt::known::no_repeat::PORT) option
  fn port(&self) -> Option<u16> {
    self.get_u16(opt::known::no_repeat::PORT)
  }

  /// Update the value for the [Uri-Path](opt::known::no_repeat::PATH) option,
  /// discarding any existing values.
  ///
  /// ```
  /// use toad_msg::alloc::Message;
  /// use toad_msg::{Code, Id, MessageOptions, Token, Type};
  ///
  /// let mut msg = Message::new(Type::Con, Code::GET, Id(1), Token(Default::default()));
  ///
  /// msg.set_host("cheese.com").unwrap();
  /// msg.set_port(1234).unwrap();
  /// msg.set_path("cheese/havarti/suggestions").unwrap();
  /// assert_eq!(msg.host(), Ok(Some("cheese.com")));
  /// assert_eq!(msg.port(), Some(1234));
  /// assert_eq!(msg.path_string(),
  ///            Ok("cheese/havarti/suggestions".to_string()));
  /// ```
  fn set_path<S>(&mut self, path: S) -> Result<(), Self::SetError>
    where S: AsRef<str>
  {
    path.as_ref()
        .split('/')
        .try_for_each(|segment| {
          self.add(opt::known::repeat::PATH,
                   segment.as_bytes().iter().copied().collect())
        })
        .map(|_| ())
  }

  /// Get an iterator over the [Uri-Path](opt::known::repeat::PATH) segments
  fn path<'a, F>(&'a self) -> Result<F, Utf8Error>
    where F: FromIterator<&'a str>
  {
    self.get_strs(opt::known::repeat::PATH)
  }

  /// Get the fully built path, joining segments with '/'.
  #[cfg(feature = "std")]
  fn path_string<'a>(&'a self) -> Result<String, Utf8Error> {
    self.get_strs::<Vec<_>>(opt::known::repeat::PATH)
        .map(|segs| {
          let mut s = segs.into_iter()
                          .fold(String::new(), |s, seg| format!("{s}{seg}/"));
          s.pop();
          s
        })
  }

  /// Insert a new value for the [Uri-Query](opt::known::repeat::QUERY) option,
  /// alongside any existing values.
  fn add_query<S>(&mut self, query: S) -> Result<(), Self::SetError>
    where S: AsRef<str>
  {
    self.add(opt::known::repeat::QUERY,
             query.as_ref().as_bytes().iter().copied().collect())
  }

  /// Get all query parameters for this request
  ///
  /// ```
  /// use toad_msg::alloc::Message;
  /// use toad_msg::{Code, Id, MessageOptions, Token, Type};
  ///
  /// let mut msg = Message::new(Type::Con, Code::GET, Id(1), Token(Default::default()));
  ///
  /// msg.add_query("id[eq]=123").unwrap();
  /// msg.add_query("price[lt]=333").unwrap();
  /// assert_eq!(msg.query::<Vec<_>>(),
  ///            Ok(vec!["id[eq]=123", "price[lt]=333"]));
  /// ```
  fn query<'a, F>(&'a self) -> Result<F, Utf8Error>
    where F: FromIterator<&'a str>
  {
    self.get_strs(opt::known::repeat::QUERY)
  }

  /// Update the value for the [Content-Format](opt::known::no_repeat::CONTENT_FORMAT) option,
  /// discarding any existing values.
  #[doc = rfc_7252_doc!("5.10.3")]
  fn set_content_format(&mut self, format: ContentFormat) -> Result<(), Self::SetError> {
    self.set(opt::known::no_repeat::CONTENT_FORMAT,
             format.into_iter().collect())
        .map(|_| ())
  }

  /// Get the value for the [Content-Format](opt::known::no_repeat::CONTENT_FORMAT) option
  ///
  /// ```
  /// use toad_msg::alloc::Message;
  /// use toad_msg::ContentFormat::Json;
  /// use toad_msg::{Code, Id, MessageOptions, Token, Type};
  ///
  /// let mut msg = Message::new(Type::Con, Code::GET, Id(1), Token(Default::default()));
  ///
  /// msg.set_content_format(Json).unwrap();
  /// assert_eq!(msg.content_format(), Some(Json));
  /// ```
  fn content_format(&self) -> Option<ContentFormat> {
    self.get_u16(opt::known::no_repeat::CONTENT_FORMAT)
        .map(ContentFormat::from)
  }

  /// Set the value for the [Observe](opt::known::no_repeat::OBSERVE) option,
  /// discarding any existing values.
  fn set_observe(&mut self, a: observe::Action) -> Result<(), Self::SetError> {
    self.set(opt::known::no_repeat::OBSERVE,
             core::iter::once(u8::from(a)).collect())
        .map(|_| ())
  }

  /// Get the value for the [Observe](opt::known::no_repeat::OBSERVE) option
  fn observe(&self) -> Option<observe::Action> {
    self.get_u8(opt::known::no_repeat::OBSERVE)
        .and_then(observe::Action::from_byte)
  }

  /// Update the value for the [Accept](opt::known::no_repeat::ACCEPT) option,
  /// discarding any existing values.
  #[doc = rfc_7252_doc!("5.10.4")]
  fn set_accept(&mut self, format: ContentFormat) -> Result<(), Self::SetError> {
    self.set(opt::known::no_repeat::ACCEPT, format.into_iter().collect())
        .map(|_| ())
  }

  /// Get the value for the [Accept](opt::known::no_repeat::ACCEPT) option
  fn accept(&self) -> Option<ContentFormat> {
    self.get_u16(opt::known::no_repeat::ACCEPT)
        .map(ContentFormat::from)
  }

  /// Update the value for the [Size1](opt::known::no_repeat::SIZE1) option,
  /// discarding any existing values.
  #[doc = rfc_7252_doc!("5.10.9")]
  fn set_size1(&mut self, size_bytes: u64) -> Result<(), Self::SetError> {
    self.set(opt::known::no_repeat::SIZE1,
             size_bytes.to_be_bytes().into_iter().collect())
        .map(|_| ())
  }

  /// Get the value for the [Size1](opt::known::no_repeat::SIZE1) option
  fn size1(&self) -> Option<u64> {
    self.get_u64(opt::known::no_repeat::SIZE1)
  }

  /// Update the value for the [Size2](opt::known::no_repeat::SIZE2) option,
  /// discarding any existing values.
  fn set_size2(&mut self, size_bytes: u64) -> Result<(), Self::SetError> {
    self.set(opt::known::no_repeat::SIZE2,
             size_bytes.to_be_bytes().into_iter().collect())
        .map(|_| ())
  }

  /// Get the value for the [Size2](opt::known::no_repeat::SIZE2) option
  fn size2(&self) -> Option<u64> {
    self.get_u64(opt::known::no_repeat::SIZE2)
  }

  /// Discard all values for [If-Match](opt::known::repeat::IF_MATCH), and replace them with
  /// an empty value.
  ///
  /// This signals that our request should only be processed if we're trying to update
  /// a resource that exists (e.g. this ensures PUT only updates and will never insert)
  #[doc = rfc_7252_doc!("5.10.8.1")]
  fn set_if_exists(&mut self) -> Result<(), Self::SetError> {
    self.set(opt::known::repeat::IF_MATCH, Default::default())
        .map(|_| ())
  }

  /// Get whether or not [`Message::set_if_exists`] applies
  fn if_exists_flag_enabled(&self) -> bool {
    self.get(opt::known::repeat::IF_MATCH)
        .map(|vs| vs.iter().any(|v| v.0.len() == 0))
        .unwrap_or(false)
  }

  /// Enable the [If-None-Match](opt::known::no_repeat::IF_NONE_MATCH) flag
  ///
  /// This signals that our request should only be processed if we're trying to insert
  /// a resource that does not exist (e.g. this ensures PUT only inserts and will never update)
  #[doc = rfc_7252_doc!("5.10.8.2")]
  fn set_if_not_exists(&mut self) -> Result<(), Self::SetError> {
    self.set(opt::known::no_repeat::IF_NONE_MATCH, Default::default())
        .map(|_| ())
  }

  /// Get whether or not [`Message::set_if_not_exists`] applies
  fn if_not_exists_flag_enabled(&self) -> bool {
    self.get_first(opt::known::no_repeat::IF_NONE_MATCH)
        .map(|_| true)
        .unwrap_or(false)
  }

  /// Update the value for the [Max-Age](opt::known::no_repeat::MAX_AGE) option,
  /// discarding any existing values.
  #[doc = rfc_7252_doc!("5.10.5")]
  fn set_max_age(&mut self, max_age_seconds: u32) -> Result<(), Self::SetError> {
    self.set(opt::known::no_repeat::MAX_AGE,
             max_age_seconds.to_be_bytes().into_iter().collect())
        .map(|_| ())
  }

  /// Get the value for the [Max-Age](opt::known::no_repeat::MAX_AGE) option, in seconds
  fn max_age_seconds(&self) -> Option<u32> {
    self.get_u32(opt::known::no_repeat::MAX_AGE)
  }

  /// Update the value for the [Proxy-Uri](opt::known::no_repeat::PROXY_URI) option,
  /// discarding any existing values.
  #[doc = rfc_7252_doc!("5.10.2")]
  fn set_proxy_uri<S>(&mut self, uri: S) -> Result<(), Self::SetError>
    where S: AsRef<str>
  {
    self.set(opt::known::no_repeat::PROXY_URI,
             uri.as_ref().as_bytes().iter().copied().collect())
        .map(|_| ())
  }

  /// Get the value for the [Proxy-Uri](opt::known::no_repeat::PROXY_URI) option
  fn proxy_uri(&self) -> Result<Option<&str>, Utf8Error> {
    self.get_str(opt::known::no_repeat::PROXY_URI)
  }

  /// Update the value for the [Proxy-Scheme](opt::known::no_repeat::PROXY_SCHEME) option,
  /// discarding any existing values.
  fn set_proxy_scheme<S>(&mut self, scheme: S) -> Result<(), Self::SetError>
    where S: AsRef<str>
  {
    self.set(opt::known::no_repeat::PROXY_SCHEME,
             scheme.as_ref().as_bytes().iter().copied().collect())
        .map(|_| ())
  }

  /// Get the value for the [Proxy-Scheme](opt::known::no_repeat::PROXY_SCHEME) option
  fn proxy_scheme(&self) -> Result<Option<&str>, Utf8Error> {
    self.get_str(opt::known::no_repeat::PROXY_SCHEME)
  }

  /// Insert a new value for the [If-Match](opt::known::repeat::IF_MATCH) option,
  /// alongside any existing values.
  #[doc = rfc_7252_doc!("5.10.8.1")]
  fn add_if_match<B>(&mut self, tag: B) -> Result<(), Self::SetError>
    where B: AsRef<[u8]>
  {
    if let Some(others) = self.remove(opt::known::repeat::IF_MATCH) {
      others.into_iter()
            .filter(|v| v.0.len() > 0)
            .map(|v| self.add(opt::known::repeat::IF_MATCH, v))
            .collect::<Result<(), _>>()?;
    }

    self.add(opt::known::repeat::IF_MATCH,
             tag.as_ref().iter().copied().collect())
  }

  /// Get all values for the [If-Match](opt::known::repeat::IF_MATCH) option
  fn if_match(&self) -> Option<&Self::OptValues> {
    self.get(opt::known::repeat::IF_MATCH)
  }

  /// Insert a new value for the [Location-Path](opt::known::repeat::LOCATION_PATH) option,
  /// alongside any existing values.
  #[doc = rfc_7252_doc!("5.10.7")]
  fn add_location_path<S>(&mut self, path: S) -> Result<(), Self::SetError>
    where S: AsRef<str>
  {
    self.add(opt::known::repeat::LOCATION_PATH,
             path.as_ref().as_bytes().iter().copied().collect())
  }

  /// Get all values for the [Location-Path](opt::known::repeat::LOCATION_PATH) option
  fn location_path<'a, F>(&'a self) -> Result<F, Utf8Error>
    where F: FromIterator<&'a str>
  {
    self.get_strs(opt::known::repeat::LOCATION_PATH)
  }

  /// Insert a new value for the [Location-Query](opt::known::repeat::LOCATION_QUERY) option,
  /// alongside any existing values.
  #[doc = rfc_7252_doc!("5.10.7")]
  fn add_location_query<S>(&mut self, query: S) -> Result<(), Self::SetError>
    where S: AsRef<str>
  {
    self.add(opt::known::repeat::LOCATION_QUERY,
             query.as_ref().as_bytes().iter().copied().collect())
  }

  /// Get all values for the [Location-Query](opt::known::repeat::LOCATION_QUERY) option
  fn location_query<'a, F>(&'a self) -> Result<F, Utf8Error>
    where F: FromIterator<&'a str>
  {
    self.get_strs(opt::known::repeat::LOCATION_QUERY)
  }

  /// Insert a new value for the [ETag](opt::known::repeat::ETAG) option,
  /// alongside any existing values.
  #[doc = rfc_7252_doc!("5.10.7")]
  fn add_etag<B>(&mut self, tag: B) -> Result<(), Self::SetError>
    where B: AsRef<[u8]>
  {
    self.add(opt::known::repeat::ETAG,
             tag.as_ref().iter().copied().collect())
  }

  /// Get all values for the [ETag](opt::known::repeat::ETAG) option
  fn etags(&self) -> Option<&Self::OptValues> {
    self.get(opt::known::repeat::ETAG)
  }
}

impl<PayloadBytes: Array<Item = u8> + AppendCopy<u8>, Options: OptionMap>
  Message<PayloadBytes, Options>
{
  /// Create a new message
  pub fn new(ty: Type, code: Code, id: Id, token: Token) -> Self {
    Self { id,
           token,
           ty,
           code,
           ver: Version::default(),
           payload: Payload(PayloadBytes::default()),
           opts: Options::default() }
  }

  /// Get the payload
  pub fn payload(&self) -> &Payload<PayloadBytes> {
    &self.payload
  }

  /// Set the payload, returning the old payload if there was one
  pub fn set_payload(&mut self, p: Payload<PayloadBytes>) -> Option<Payload<PayloadBytes>> {
    let mut old: Payload<_> = p;
    core::mem::swap(&mut old, &mut self.payload);
    Some(old).filter(|old| old.0.len() > 0)
  }

  /// Create a new message that ACKs this one.
  ///
  /// This needs an [`Id`] to assign to the newly created message.
  ///
  /// ```
  /// // we are a server
  ///
  /// use std::net::SocketAddr;
  ///
  /// use toad_msg::alloc::Message;
  /// use toad_msg::Id;
  ///
  /// fn server_get_request() -> Option<(SocketAddr, Message)> {
  ///   // Servery sockety things...
  ///   # use std::net::{Ipv4Addr, ToSocketAddrs};
  ///   # use toad_msg::{Type, Code, Token, Version, Payload};
  ///   # let addr = (Ipv4Addr::new(0, 0, 0, 0), 1234);
  ///   # let addr = addr.to_socket_addrs().unwrap().next().unwrap();
  ///   # let msg = Message { code: Code::new(0, 0),
  ///   #                     id: Id(1),
  ///   #                     ty: Type::Con,
  ///   #                     ver: Version(1),
  ///   #                     token: Token(tinyvec::array_vec!([u8; 8] => 254)),
  ///   #                     opts: Default::default(),
  ///   #                     payload: Payload(vec![]) };
  ///   # Some((addr, msg))
  /// }
  ///
  /// fn server_send_msg(addr: SocketAddr, msg: Message) -> Result<(), ()> {
  ///   // Message sendy bits...
  ///   # Ok(())
  /// }
  ///
  /// let (addr, req) = server_get_request().unwrap();
  /// let ack_id = Id(req.id.0 + 1);
  /// let ack = req.ack(ack_id);
  ///
  /// server_send_msg(addr, ack).unwrap();
  /// ```
  pub fn ack(&self, id: Id) -> Self {
    Self { id,
           token: self.token,
           ver: Default::default(),
           ty: Type::Ack,
           code: Code::new(0, 0),
           payload: Payload(Default::default()),
           opts: Default::default() }
  }

  fn add(&mut self,
         n: OptNumber,
         v: OptValue<Options::OptValue>)
         -> Result<(), SetOptionError<OptValue<Options::OptValue>, Options::OptValues>> {
    match (self.remove(n).unwrap_or_default(), &mut self.opts) {
      | (vals, _) if vals.is_full() => Err(SetOptionError::RepeatedTooManyTimes(v)),
      | (vals, opts) if opts.is_full() => Err(SetOptionError::TooManyOptions(n, vals)),
      | (mut vals, opts) => {
        vals.push(v);
        opts.insert(n, vals).ok();
        Ok(())
      },
    }
  }

  fn set(
    &mut self,
    n: OptNumber,
    v: OptValue<Options::OptValue>)
    -> Result<Option<Options::OptValues>,
              SetOptionError<OptValue<Options::OptValue>, Options::OptValues>> {
    Ok(self.remove(n)).and_then(|old| self.add(n, v).map(|_| old))
  }

  fn count(&self, n: OptNumber) -> usize {
    self.get(n).map(|a| a.len()).unwrap_or(0)
  }

  fn get(&self, n: OptNumber) -> Option<&Options::OptValues> {
    self.opts.get(&n)
  }

  fn get_first(&self, n: OptNumber) -> Option<&OptValue<Options::OptValue>> {
    self.get(n).and_then(|vs| vs.get(0))
  }

  fn get_str(&self, n: OptNumber) -> Result<Option<&str>, Utf8Error> {
    match self.get_first(n) {
      | Some(v) => from_utf8(&v.0).map(Some),
      | _ => Ok(None),
    }
  }

  fn get_strs<'a, F>(&'a self, n: OptNumber) -> Result<F, Utf8Error>
    where F: FromIterator<&'a str>
  {
    match self.get(n) {
      | Some(vs) if vs.len() >= 1 => vs.iter().map(|s| from_utf8(&s.0)).collect(),
      | _ => Ok(core::iter::empty().collect()),
    }
  }

  fn get_u8(&self, n: OptNumber) -> Option<u8> {
    self.get_first(n)
        .filter(|bytes| bytes.0.len() == 1)
        .map(|bytes| bytes.0[0])
  }

  fn get_u16(&self, n: OptNumber) -> Option<u16> {
    self.get_first(n)
        .filter(|bytes| bytes.0.len() == 2)
        .map(|bytes| u16::from_be_bytes([bytes.0[0], bytes.0[1]]))
  }

  fn get_u32(&self, n: OptNumber) -> Option<u32> {
    self.get_first(n)
        .filter(|bytes| bytes.0.len() == 4)
        .map(|bytes| u32::from_be_bytes([bytes.0[0], bytes.0[1], bytes.0[2], bytes.0[3]]))
  }

  fn get_u64(&self, n: OptNumber) -> Option<u64> {
    self.get_first(n)
        .filter(|bytes| bytes.0.len() == 8)
        .map(|bytes| {
          u64::from_be_bytes([bytes.0[0], bytes.0[1], bytes.0[2], bytes.0[3], bytes.0[4],
                              bytes.0[5], bytes.0[6], bytes.0[7]])
        })
  }

  fn remove(&mut self, n: OptNumber) -> Option<Options::OptValues> {
    self.opts.remove(&n)
  }
}

impl<Bytes: AsRef<[u8]>, PayloadBytes: Array<Item = u8> + AppendCopy<u8>, Options: OptionMap>
  TryFromBytes<Bytes> for Message<PayloadBytes, Options>
{
  type Error = MessageParseError;

  fn try_from_bytes(bytes: Bytes) -> Result<Self, Self::Error> {
    let mut bytes = Cursor::new(bytes);

    let Byte1 { tkl, ty, ver } = bytes.next()
                                      .ok_or_else(MessageParseError::eof)?
                                      .try_into()?;

    if tkl > 8 {
      return Err(Self::Error::InvalidTokenLength(tkl));
    }

    let code: Code = bytes.next().ok_or_else(MessageParseError::eof)?.into();
    let id: Id = Id::try_consume_bytes(&mut bytes)?;

    let token = bytes.take_exact(tkl as usize)
                     .ok_or_else(MessageParseError::eof)?;
    let token = tinyvec::ArrayVec::<[u8; 8]>::try_from(token).expect("tkl was checked to be <= 8");
    let token = Token(token);

    let opts = Options::try_consume_bytes(&mut bytes).map_err(Self::Error::OptParseError)?;

    let mut payload = PayloadBytes::reserve(bytes.remaining());
    payload.append_copy(bytes.take_until_end());
    let payload = Payload(payload);

    Ok(Message { id,
                 ty,
                 ver,
                 code,
                 token,
                 opts,
                 payload })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::alloc;

  #[test]
  fn parse_msg() {
    let (expect, msg) = crate::test_msg();
    assert_eq!(alloc::Message::try_from_bytes(&msg).unwrap(), expect)
  }

  #[test]
  fn parse_byte1() {
    let byte = 0b_01_10_0011u8;
    let byte = Byte1::try_from(byte).unwrap();
    assert_eq!(byte,
               Byte1 { ver: Version(1),
                       ty: Type::Ack,
                       tkl: 3 })
  }

  #[test]
  fn parse_id() {
    let mut id_bytes = Cursor::new(34u16.to_be_bytes());
    let id = Id::try_consume_bytes(&mut id_bytes).unwrap();
    assert_eq!(id, Id(34));
  }
}
