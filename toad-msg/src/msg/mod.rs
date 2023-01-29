use toad_common::{AppendCopy, Array, Cursor, GetSize};
use toad_macros::rfc_7252_doc;

use crate::content_format::ContentFormat;
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
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Payload<C>(pub C);

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

impl<PayloadBytes: Array<Item = u8>, Options: OptionMap> GetSize
  for Message<PayloadBytes, Options>
{
  fn get_size(&self) -> usize {
    let header_size = 4;
    let payload_marker_size = 1;
    let payload_size = self.payload.0.get_size();
    let token_size = self.token.0.len();
    let opts_size: usize = self.opts.opt_refs().map(|o| o.get_size()).sum();

    header_size + payload_marker_size + payload_size + token_size + opts_size
  }

  fn max_size(&self) -> Option<usize> {
    None
  }

  fn is_full(&self) -> bool {
    false
  }
}

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
/// use std::collections::BTreeMap;
///
/// use toad_msg::TryFromBytes;
/// use toad_msg::*;
///
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
/// // `toad_msg::alloc::Message` uses `Vec` as the backing structure for byte buffers
/// let msg = toad_msg::alloc::Message::try_from_bytes(packet.clone()).unwrap();
/// let mut opts_expected = BTreeMap::from([(OptNumber(12), vec![OptValue(content_format.iter().map(|u| *u).collect())])]);
///
/// let expected = toad_msg::alloc::Message {
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
#[derive(Clone, PartialEq, PartialOrd, Eq, Ord, Debug)]
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

/// An error occurred during a call to [`Message::set`]
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SetOptionError<OV, OVs> {
  RepeatedTooManyTimes(OV),
  TooManyOptions(OptNumber, OVs),
}

impl<PayloadBytes: Array<Item = u8> + AppendCopy<u8>, Options: OptionMap>
  Message<PayloadBytes, Options>
{
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

  /// Insert a new value for a given option
  ///
  /// Errors when there cannot be any more options, or the option
  /// cannot be repeated any more (only applies to non-std environments)
  #[doc = rfc_7252_doc!("5.4.5")]
  pub fn add(&mut self,
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

  /// Replace any / all existing values with a new one,
  /// yielding the previous value(s)
  pub fn set(
    &mut self,
    n: OptNumber,
    v: OptValue<Options::OptValue>)
    -> Result<Option<Options::OptValues>,
              SetOptionError<OptValue<Options::OptValue>, Options::OptValues>> {
    Ok(self.remove(n)).and_then(|old| self.add(n, v).map(|_| old))
  }

  /// Get the number of values for a given option
  pub fn count(&self, n: OptNumber) -> usize {
    self.get(n).map(|a| a.get_size()).unwrap_or(0)
  }

  /// Get the value(s) of an option by number
  ///
  /// This just invokes [`toad_common::Map::get`] on [`Message.opts`].
  pub fn get(&self, n: OptNumber) -> Option<&Options::OptValues> {
    self.opts.get(&n)
  }

  /// Remove all values for the option from this message,
  /// returning them if there were any.
  pub fn remove(&mut self, n: OptNumber) -> Option<Options::OptValues> {
    self.opts.remove(&n)
  }

  /// Update the value for the [Uri-Host](opt::known::no_repeat::HOST) option,
  /// discarding any existing values.
  #[doc = rfc_7252_doc!("5.10.1")]
  pub fn set_host<S>(
    &mut self,
    host: S)
    -> Result<(), SetOptionError<OptValue<Options::OptValue>, Options::OptValues>>
    where S: AsRef<str>
  {
    self.set(opt::known::no_repeat::HOST,
             host.as_ref().as_bytes().iter().copied().collect())
        .map(|_| ())
  }

  /// Update the value for the [Uri-Port](opt::known::no_repeat::PORT) option,
  /// discarding any existing values.
  #[doc = rfc_7252_doc!("5.10.1")]
  pub fn set_port(
    &mut self,
    port: u16)
    -> Result<(), SetOptionError<OptValue<Options::OptValue>, Options::OptValues>> {
    self.set(opt::known::no_repeat::PORT,
             port.to_be_bytes().into_iter().collect())
        .map(|_| ())
  }

  /// Update the value for the [Uri-Path](opt::known::no_repeat::PATH) option,
  /// discarding any existing values.
  #[doc = rfc_7252_doc!("5.10.1")]
  pub fn set_path<S>(
    &mut self,
    path: S)
    -> Result<(), SetOptionError<OptValue<Options::OptValue>, Options::OptValues>>
    where S: AsRef<str>
  {
    self.set(opt::known::no_repeat::PATH,
             path.as_ref().as_bytes().iter().copied().collect())
        .map(|_| ())
  }

  /// Insert a new value for the [Uri-Query](opt::known::repeat::QUERY) option,
  /// alongside any existing values.
  #[doc = rfc_7252_doc!("5.10.1")]
  pub fn add_query<S>(
    &mut self,
    query: S)
    -> Result<(), SetOptionError<OptValue<Options::OptValue>, Options::OptValues>>
    where S: AsRef<str>
  {
    self.add(opt::known::repeat::QUERY,
             query.as_ref().as_bytes().iter().copied().collect())
  }

  /// Update the value for the [Content-Format](opt::known::no_repeat::CONTENT_FORMAT) option,
  /// discarding any existing values.
  #[doc = rfc_7252_doc!("5.10.3")]
  pub fn set_content_format(
    &mut self,
    format: ContentFormat)
    -> Result<(), SetOptionError<OptValue<Options::OptValue>, Options::OptValues>> {
    self.set(opt::known::no_repeat::CONTENT_FORMAT,
             format.into_iter().collect())
        .map(|_| ())
  }

  /// Update the value for the [Accept](opt::known::no_repeat::ACCEPT) option,
  /// discarding any existing values.
  #[doc = rfc_7252_doc!("5.10.4")]
  pub fn set_accept(
    &mut self,
    format: ContentFormat)
    -> Result<(), SetOptionError<OptValue<Options::OptValue>, Options::OptValues>> {
    self.set(opt::known::no_repeat::ACCEPT, format.into_iter().collect())
        .map(|_| ())
  }

  /// Update the value for the [Size1](opt::known::no_repeat::SIZE1) option,
  /// discarding any existing values.
  #[doc = rfc_7252_doc!("5.10.9")]
  pub fn set_size1(
    &mut self,
    size_bytes: u64)
    -> Result<(), SetOptionError<OptValue<Options::OptValue>, Options::OptValues>> {
    self.set(opt::known::no_repeat::SIZE1,
             size_bytes.to_be_bytes().into_iter().collect())
        .map(|_| ())
  }

  /// Discard all values for [If-Match](opt::known::repeat::IF_MATCH), and replace them with
  /// an empty value.
  ///
  /// This signals that our request should only be processed if we're trying to update
  /// a resource that exists (e.g. this ensures PUT only updates and will never insert)
  #[doc = rfc_7252_doc!("5.10.8.1")]
  pub fn set_if_exists(
    &mut self)
    -> Result<(), SetOptionError<OptValue<Options::OptValue>, Options::OptValues>> {
    self.set(opt::known::repeat::IF_MATCH, Default::default())
        .map(|_| ())
  }

  /// Enable the [If-None-Match](opt::known::no_repeat::IF_NONE_MATCH) flag
  ///
  /// This signals that our request should only be processed if we're trying to insert
  /// a resource that does not exist (e.g. this ensures PUT only inserts and will never update)
  #[doc = rfc_7252_doc!("5.10.8.2")]
  pub fn set_if_not_exists(
    &mut self)
    -> Result<(), SetOptionError<OptValue<Options::OptValue>, Options::OptValues>> {
    self.set(opt::known::no_repeat::IF_NONE_MATCH, Default::default())
        .map(|_| ())
  }

  /// Update the value for the [Max-Age](opt::known::no_repeat::MAX_AGE) option,
  /// discarding any existing values.
  #[doc = rfc_7252_doc!("5.10.5")]
  pub fn set_max_age(
    &mut self,
    max_age_seconds: u32)
    -> Result<(), SetOptionError<OptValue<Options::OptValue>, Options::OptValues>> {
    self.set(opt::known::no_repeat::MAX_AGE,
             max_age_seconds.to_be_bytes().into_iter().collect())
        .map(|_| ())
  }

  /// Update the value for the [Proxy-Uri](opt::known::no_repeat::PROXY_URI) option,
  /// discarding any existing values.
  #[doc = rfc_7252_doc!("5.10.2")]
  pub fn set_proxy_uri<S>(
    &mut self,
    uri: S)
    -> Result<(), SetOptionError<OptValue<Options::OptValue>, Options::OptValues>>
    where S: AsRef<str>
  {
    self.set(opt::known::no_repeat::PROXY_URI,
             uri.as_ref().as_bytes().iter().copied().collect())
        .map(|_| ())
  }

  /// Update the value for the [Proxy-Scheme](opt::known::no_repeat::PROXY_SCHEME) option,
  /// discarding any existing values.
  #[doc = rfc_7252_doc!("5.10.2")]
  pub fn set_proxy_scheme<S>(
    &mut self,
    scheme: S)
    -> Result<(), SetOptionError<OptValue<Options::OptValue>, Options::OptValues>>
    where S: AsRef<str>
  {
    self.set(opt::known::no_repeat::PROXY_URI,
             scheme.as_ref().as_bytes().iter().copied().collect())
        .map(|_| ())
  }

  /// Insert a new value for the [If-Match](opt::known::repeat::IF_MATCH) option,
  /// alongside any existing values.
  #[doc = rfc_7252_doc!("5.10.8.1")]
  pub fn add_if_match<B>(
    &mut self,
    tag: B)
    -> Result<(), SetOptionError<OptValue<Options::OptValue>, Options::OptValues>>
    where B: AsRef<[u8]>
  {
    if let Some(others) = self.remove(opt::known::repeat::IF_MATCH) {
      others.into_iter()
            .filter(|v| v.0.get_size() > 0)
            .map(|v| self.add(opt::known::repeat::IF_MATCH, v))
            .collect::<Result<(), _>>()?;
    }

    self.add(opt::known::repeat::IF_MATCH,
             tag.as_ref().iter().copied().collect())
  }

  /// Insert a new value for the [Location-Path](opt::known::repeat::LOCATION_PATH) option,
  /// alongside any existing values.
  #[doc = rfc_7252_doc!("5.10.7")]
  pub fn add_location_path<S>(
    &mut self,
    path: S)
    -> Result<(), SetOptionError<OptValue<Options::OptValue>, Options::OptValues>>
    where S: AsRef<str>
  {
    self.add(opt::known::repeat::LOCATION_PATH,
             path.as_ref().as_bytes().iter().copied().collect())
  }

  /// Insert a new value for the [Location-Query](opt::known::repeat::LOCATION_QUERY) option,
  /// alongside any existing values.
  #[doc = rfc_7252_doc!("5.10.7")]
  pub fn add_location_query<S>(
    &mut self,
    query: S)
    -> Result<(), SetOptionError<OptValue<Options::OptValue>, Options::OptValues>>
    where S: AsRef<str>
  {
    self.add(opt::known::repeat::LOCATION_QUERY,
             query.as_ref().as_bytes().iter().copied().collect())
  }

  /// Insert a new value for the [ETag](opt::known::repeat::ETAG) option,
  /// alongside any existing values.
  #[doc = rfc_7252_doc!("5.10.7")]
  pub fn add_etag<B>(
    &mut self,
    tag: B)
    -> Result<(), SetOptionError<OptValue<Options::OptValue>, Options::OptValues>>
    where B: AsRef<[u8]>
  {
    self.add(opt::known::repeat::ETAG,
             tag.as_ref().iter().copied().collect())
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
