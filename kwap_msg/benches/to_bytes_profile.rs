use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use kwap_msg::alloc::*;

#[path = "./to_bytes.rs"]
mod to_bytes;

use to_bytes::BenchInput;

fn message_to_bytes_profile(c: &mut Criterion) {
  let f = |bi: &BenchInput| -> Vec<u8> { Into::<Message>::into(bi).into() };
  let inp = BenchInput {tkl: 8, n_opts: 16, opt_size: 32, payload_size: 512};
  let bytes: Vec<u8> = f(&inp).into();
  let coap_lite_packet = coap_lite::Packet::from_bytes(&bytes).unwrap();
  c.bench_with_input(BenchmarkId::new("message_to_bytes", "profile"), &inp, |b, inp| b.iter(|| f(inp)));
  c.bench_with_input(BenchmarkId::new("message_to_bytes", "coap_lite"), &coap_lite_packet, |b, packet| b.iter(|| packet.to_bytes()));
}

criterion_group!(benches, message_to_bytes_profile);
criterion_main!(benches);
