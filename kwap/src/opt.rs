use kwap_msg::{Collection, OptNumber, OptValue};

/// # CoAP Option
#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub struct Opt<C: Collection<u8>>
  where for<'a> &'a C: IntoIterator<Item = &'a u8>
{
  /// See [`OptNumber`]
  pub number: OptNumber,

  /// See [`OptValue`]
  pub value: OptValue<C>,
}

///
pub trait ToKwapMsgOpts<C: Collection<u8>, Dest: Collection<kwap_msg::Opt<C>>>
  where for<'a> &'a C: IntoIterator<Item = &'a u8>,
  for<'a> &'a Dest: IntoIterator<Item = &'a kwap_msg::Opt<C>>
    {
  fn to_kwap_msg_opts(self) -> Dest;
}

impl<C: Collection<u8>, Src: Collection<Opt<C>>, Dest: Collection<kwap_msg::Opt<C>>> ToKwapMsgOpts<C, Dest> for Src
  where for<'a> &'a C: IntoIterator<Item = &'a u8>,
  for<'a> &'a Src: IntoIterator<Item = &'a Opt<C>>,
  for<'a> &'a Dest: IntoIterator<Item = &'a kwap_msg::Opt<C>>,
    {
  fn to_kwap_msg_opts(self) -> Dest {todo!()}
}
