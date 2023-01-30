use tinyvec::ArrayVec;
use std::collections::BTreeMap;
use toad_msg::*;

#[path = "common.rs"]
mod common;
use common::*;

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
  pub fn get_alloc_message(&self) -> alloc::Message {
    self.into()
  }
  pub fn get_no_alloc_message<const P: usize, const N: usize, const O: usize>(
    &self)
-> StackMessage<P, N, O> {
    self.into()
  }
  pub fn get_coap_lite_packet(&self) -> coap_lite::Packet {
    coap_lite::Packet::from_bytes(&self.get_bytes()).unwrap()
  }
}

impl<'a> From<&'a TestInput> for alloc::Message {
  fn from(inp: &'a TestInput) -> alloc::Message {
    let opts: BTreeMap<_, _> =
      (0..inp.n_opts).map(|n| (OptNumber(n as u32), vec![OptValue(core::iter::repeat(1).take(inp.opt_size)
                                                                         .collect())]))
                     .collect();

    let token = core::iter::repeat(1u8).take(inp.tkl as _)
                                       .collect::<tinyvec::ArrayVec<[_; 8]>>();

    alloc::Message { id: Id(1),
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
for StackMessage<P, N, O>
{
fn from(inp: &'a TestInput) -> StackMessage<P, N, O> {
    let opts: ArrayVec<[_; N]> =
      (0..inp.n_opts).map(|n| (OptNumber(n as u32), tinyvec::array_vec![_ => OptValue(core::iter::repeat(1).take(inp.opt_size)
                                                                         .collect())]))
                     .collect();

    let token = core::iter::repeat(1u8).take(inp.tkl as _)
                                       .collect::<tinyvec::ArrayVec<[_; 8]>>();

StackMessage { id: Id(1),
                      ty: Type::Non,
                      ver: Default::default(),
                      token: Token(token),
                      code: Code { class: 2,
                                   detail: 5 },
                      opts,
                      payload: Payload(core::iter::repeat(1u8).take(inp.payload_size).collect()) }
  }
}
