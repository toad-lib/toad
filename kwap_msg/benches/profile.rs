use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use kwap_msg::*;
use tinyvec::ArrayVec;

#[path = "bench_input.rs"]
mod bench_input;
use bench_input::TestInput;

fn profile(c: &mut Criterion) {
  type ArrayMessage = ArrayVecMessage<1024, 16, 128>;
  let inp = TestInput { tkl: 8,
                        n_opts: 16,
                        opt_size: 128,
                        payload_size: 1024 };

  let bytes: Vec<u8> = inp.get_bytes();
  let coap_lite_packet = coap_lite::Packet::from_bytes(&bytes).unwrap();

  c.bench_with_input(BenchmarkId::new("msg/profile/to_bytes", "kwap_msg/alloc"),
                     &inp,
                     |b, inp| {
                       b.iter_batched(|| VecMessage::from(inp),
                                      |m| m.try_into_bytes::<Vec<u8>>().unwrap(),
                                      BatchSize::SmallInput)
                     });
  c.bench_with_input(BenchmarkId::new("msg/profile/to_bytes", "kwap_msg/no_alloc"),
                     &inp,
                     |b, inp| {
                       b.iter_batched(|| ArrayMessage::from(inp),
                                      |msg| msg.try_into_bytes::<ArrayVec<[u8; 3120]>>(),
                                      BatchSize::SmallInput)
                     });
  c.bench_with_input(BenchmarkId::new("msg/profile/to_bytes", "coap_lite"),
                     &coap_lite_packet,
                     |b, packet| b.iter(|| packet.to_bytes()));

  c.bench_with_input(BenchmarkId::new("msg/profile/from_bytes", "kwap_msg/alloc"),
                     &bytes,
                     |b, bytes| b.iter(|| VecMessage::try_from_bytes(bytes)));
  c.bench_with_input(BenchmarkId::new("msg/profile/from_bytes", "kwap_msg/no_alloc"),
                     &bytes,
                     |b, bytes| b.iter(|| ArrayMessage::try_from_bytes(bytes)));
  c.bench_with_input(BenchmarkId::new("msg/profile/from_bytes", "coap_lite"),
                     &bytes,
                     |b, bytes| b.iter(|| coap_lite::Packet::from_bytes(bytes)));
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(100).warm_up_time(std::time::Duration::from_secs(15))
           .measurement_time(std::time::Duration::from_secs(15));
    targets = profile
}
criterion_main!(benches);
