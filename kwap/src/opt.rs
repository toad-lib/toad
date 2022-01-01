use kwap_common::Array;
use kwap_msg::{OptNumber, OptValue};

/// # CoAP Option
#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub struct Opt<Bytes: Array<u8>>
  where for<'a> &'a Bytes: IntoIterator<Item = &'a u8>
{
  /// See [`OptNumber`]
  pub number: OptNumber,

  /// See [`OptValue`]
  pub value: OptValue<Bytes>,
}

///
pub trait ToKwapMsgOpts<Bytes: Array<u8>, Dest: Array<kwap_msg::Opt<Bytes>>>
  where for<'a> &'a Bytes: IntoIterator<Item = &'a u8>,
  for<'a> &'a Dest: IntoIterator<Item = &'a kwap_msg::Opt<Bytes>>
    {
  fn to_kwap_msg_opts(self) -> Dest;
}

impl<Bytes: Array<u8>, Src: Array<Opt<Bytes>>, Dest: Array<kwap_msg::Opt<Bytes>>> ToKwapMsgOpts<Bytes, Dest> for Src
  where for<'a> &'a Bytes: IntoIterator<Item = &'a u8>,
  for<'a> &'a Src: IntoIterator<Item = &'a Opt<Bytes>>,
  for<'a> &'a Dest: IntoIterator<Item = &'a kwap_msg::Opt<Bytes>>,
    {
  fn to_kwap_msg_opts(self) -> Dest {todo!()}
}
