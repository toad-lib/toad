use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};

#[path = "bench_input.rs"]
mod bench_input;
use bench_input::TestInput;
use tinyvec::ArrayVec;
use toad_msg::TryIntoBytes;

fn message_to_bytes(c: &mut Criterion) {
  let mut group = c.benchmark_group("msg/to_bytes");
  group.measurement_time(std::time::Duration::from_secs(5));

  let inputs = vec![TestInput { tkl: 0,
                                n_opts: 0,
                                opt_size: 0,
                                payload_size: 0 },
                    TestInput { tkl: 4,
                                n_opts: 4,
                                opt_size: 8,
                                payload_size: 16 },
                    TestInput { tkl: 4,
                                n_opts: 4,
                                opt_size: 16,
                                payload_size: 16 },
                    TestInput { tkl: 4,
                                n_opts: 8,
                                opt_size: 32,
                                payload_size: 16 },
                    TestInput { tkl: 8,
                                n_opts: 8,
                                opt_size: 64,
                                payload_size: 16 },
                    TestInput { tkl: 8,
                                n_opts: 8,
                                opt_size: 64,
                                payload_size: 32 },
                    TestInput { tkl: 8,
                                n_opts: 8,
                                opt_size: 64,
                                payload_size: 128 },
                    TestInput { tkl: 8,
                                n_opts: 16,
                                opt_size: 64,
                                payload_size: 128 },
                    TestInput { tkl: 8,
                                n_opts: 16,
                                opt_size: 64,
                                payload_size: 512 },
                    TestInput { tkl: 8,
                                n_opts: 32,
                                opt_size: 64,
                                payload_size: 512 },
                    TestInput { tkl: 8,
                                n_opts: 32,
                                opt_size: 64,
                                payload_size: 2048 },
                    TestInput { tkl: 8,
                                n_opts: 32,
                                opt_size: 256,
                                payload_size: 2048 },
                    TestInput { tkl: 8,
                                n_opts: 32,
                                opt_size: 512,
                                payload_size: 2048 },
                    TestInput { tkl: 8,
                                n_opts: 32,
                                opt_size: 512,
                                payload_size: 4096 },];

  for inp in inputs.iter() {
    let bytes = inp.get_bytes();

    group.bench_with_input(BenchmarkId::new("toad_msg/alloc/size", bytes.len()),
                           inp,
                           |b, inp| {
                             b.iter_batched(|| inp.get_alloc_message(),
                                            |m| m.try_into_bytes::<Vec<_>>().unwrap(),
                                            BatchSize::SmallInput)
                           });

    group.bench_with_input(BenchmarkId::new("toad_msg/no_alloc/size", bytes.len()),
                           inp,
                           |b, inp| {
                             b.iter_batched(|| inp.get_no_alloc_message::<4096, 32, 512>(),
                                            |msg| msg.try_into_bytes::<ArrayVec<[u8; 20608]>>(),
                                            BatchSize::SmallInput)
                           });

    let cl_packet = inp.get_coap_lite_packet();
    group.bench_with_input(BenchmarkId::new("coap_lite/size", bytes.len()),
                           &cl_packet,
                           |b, inp| b.iter(|| inp.to_bytes()));
  }
  group.finish();
}

criterion_group!(benches, message_to_bytes);
criterion_main!(benches);
