use tinyvec::ArrayVec;
use toad_msg::*;

#[derive(Debug, PartialEq, PartialOrd, Ord, Eq)]
pub struct TestInput {
  pub tkl: u8,
  pub n_opts: usize,
  pub opt_size: usize,
  pub payload_size: usize,
}

impl TestInput {
  pub fn get_bytes(&self) -> Vec<u8> {
    self.get_alloc_message().try_into_bytes::<Vec<_>>().unwrap()
  }
  pub fn get_alloc_message(&self) -> VecMessage {
    self.into()
  }
  pub fn get_no_alloc_message<const P: usize, const N: usize, const O: usize>(
    &self)
    -> ArrayVecMessage<P, N, O> {
    self.into()
  }
  pub fn get_coap_lite_packet(&self) -> coap_lite::Packet {
    coap_lite::Packet::from_bytes(&self.get_bytes()).unwrap()
  }
}

impl<'a> From<&'a TestInput> for VecMessage {
  fn from(inp: &'a TestInput) -> VecMessage {
    let opts: Vec<_> =
      (0..inp.n_opts).map(|n| Opt { delta: OptDelta(n as _),
                                    value: OptValue(core::iter::repeat(1).take(inp.opt_size)
                                                                         .collect()) })
                     .collect();

    let token = core::iter::repeat(1u8).take(inp.tkl as _)
                                       .collect::<tinyvec::ArrayVec<[_; 8]>>();

    VecMessage { id: Id(1),
                 ty: Type::Non,
                 ver: Default::default(),
                 token: Token(token),
                 code: Code { class: 2,
                              detail: 5 },
                 opts,
                 payload: Payload(core::iter::repeat(1u8).take(inp.payload_size).collect()) }
  }
}

impl<'a, const P: usize, const N: usize, const O: usize> From<&'a TestInput>
  for ArrayVecMessage<P, N, O>
{
  fn from(inp: &'a TestInput) -> ArrayVecMessage<P, N, O> {
    let opts: ArrayVec<[_; N]> =
      (0..inp.n_opts).map(|n| Opt::<_> { delta: OptDelta(n as _),
                                         value:
                                           OptValue::<_>(core::iter::repeat(1).take(inp.opt_size)
                                                                              .collect()) })
                     .collect();

    let token = core::iter::repeat(1u8).take(inp.tkl as _)
                                       .collect::<tinyvec::ArrayVec<[_; 8]>>();

    ArrayVecMessage { id: Id(1),
                      ty: Type::Non,
                      ver: Default::default(),
                      token: Token(token),
                      code: Code { class: 2,
                                   detail: 5 },
                      opts,
                      payload: Payload(core::iter::repeat(1u8).take(inp.payload_size).collect()) }
  }
}
