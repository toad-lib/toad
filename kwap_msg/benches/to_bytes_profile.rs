use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use kwap_msg::alloc::*;

#[path = "./to_bytes.rs"]
mod to_bytes;

use to_bytes::BenchInput;

fn message_to_bytes_profile(c: &mut Criterion) {
  let f = |bi: &BenchInput| -> Vec<u8> { Into::<Message>::into(bi).into() };
  let inp = BenchInput { tkl: 8,
                         n_opts: 16,
                         opt_size: 128,
                         payload_size: 1024 };
  let bytes: Vec<u8> = f(&inp).into();
  let coap_lite_packet = coap_lite::Packet::from_bytes(&bytes).unwrap();
  c.bench_with_input(BenchmarkId::new("msg/to_bytes/profile", "kwap_msg"), &inp, |b, inp| {
     b.iter(|| f(inp))
   });
  c.bench_with_input(BenchmarkId::new("msg/to_bytes/profile", "coap_lite"),
                     &coap_lite_packet,
                     |b, packet| b.iter(|| packet.to_bytes()));
}

criterion_group!(benches, message_to_bytes_profile);
criterion_main!(benches);
