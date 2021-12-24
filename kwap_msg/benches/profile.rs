use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use kwap_msg::{alloc::*, no_alloc, TryFromBytes, TryIntoBytes};

#[path = "bench_input.rs"]
mod bench_input;
use bench_input::BenchInput;

fn into<A: Into<B>, B>(a: A) -> B {
  a.into()
}

fn profile(c: &mut Criterion) {
  type NoAllocMessage = no_alloc::Message<1024, 16, 128>;
  let inp = BenchInput { tkl: 8,
                         n_opts: 16,
                         opt_size: 128,
                         payload_size: 1024 };

  let bytes: Vec<u8> = into::<_, Message>(&inp).into();
  let coap_lite_packet = coap_lite::Packet::from_bytes(&bytes).unwrap();

  c.bench_with_input(BenchmarkId::new("msg/profile/to_bytes", "kwap_msg/alloc"),
                     &inp,
                     |b, inp| {
                       b.iter_batched(|| into::<_, Message>(inp),
                                      |msg| into::<_, Vec<u8>>(msg),
                                      BatchSize::SmallInput)
                     });
  c.bench_with_input(BenchmarkId::new("msg/profile/to_bytes", "kwap_msg/no_alloc"),
                     &inp,
                     |b, inp| {
                       b.iter_batched(|| into::<_, NoAllocMessage>(inp),
                                      |msg| msg.try_into_bytes::<3120>(),
                                      BatchSize::SmallInput)
                     });
  c.bench_with_input(BenchmarkId::new("msg/profile/to_bytes", "coap_lite"),
                     &coap_lite_packet,
                     |b, packet| b.iter(|| packet.to_bytes()));

  c.bench_with_input(BenchmarkId::new("msg/profile/from_bytes", "kwap_msg/alloc"),
                     &bytes,
                     |b, bytes| b.iter(|| Message::try_from_bytes(bytes)));
  c.bench_with_input(BenchmarkId::new("msg/profile/from_bytes", "kwap_msg/no_alloc"),
                     &bytes,
                     |b, bytes| b.iter(|| NoAllocMessage::try_from_bytes(bytes)));
  c.bench_with_input(BenchmarkId::new("msg/profile/from_bytes", "coap_lite"),
                     &bytes,
                     |b, bytes| b.iter(|| coap_lite::Packet::from_bytes(&bytes)));
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(100).warm_up_time(std::time::Duration::from_secs(15))
           .measurement_time(std::time::Duration::from_secs(15));
    targets = profile
}
criterion_main!(benches);
