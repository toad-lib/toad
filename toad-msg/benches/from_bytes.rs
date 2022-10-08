use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use toad_msg::*;

#[path = "bench_input.rs"]
mod bench_input;
use bench_input::TestInput;

fn message_from_bytes(c: &mut Criterion) {
  let mut group = c.benchmark_group("msg/from_bytes");
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

  type ArrayMessage = ArrayVecMessage<4096, 32, 512>;

  for inp in inputs.iter() {
    let bytes = inp.get_bytes();

    group.bench_with_input(BenchmarkId::new("toad_msg/alloc/size", bytes.len()),
                           &bytes,
                           |b, bytes| b.iter(|| VecMessage::try_from_bytes(bytes)));

    group.bench_with_input(BenchmarkId::new("toad_msg/no_alloc/size", bytes.len()),
                           &bytes,
                           |b, bytes| b.iter(|| ArrayMessage::try_from_bytes(bytes)));

    group.bench_with_input(BenchmarkId::new("coap_lite/size", bytes.len()),
                           &bytes,
                           |b, bytes| b.iter(|| coap_lite::Packet::from_bytes(bytes)));
  }
  group.finish();
}

criterion_group!(benches, message_from_bytes);
criterion_main!(benches);
