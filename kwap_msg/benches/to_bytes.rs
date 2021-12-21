use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use kwap_msg::alloc::*;

#[derive(Debug, PartialEq, PartialOrd, Ord, Eq)]
pub struct BenchInput {
  pub tkl: u8,
  pub n_opts: usize,
  pub opt_size: usize,
  pub payload_size: usize,
}

impl<'a> Into<Message> for &'a BenchInput {
  fn into(self) -> Message {
    let opts: Vec<Opt> = (0..self.n_opts).map(|n| Opt { delta: OptDelta(n as _),
                                                        value: OptValue(core::iter::repeat(1).take(self.opt_size)
                                                                                             .collect()) })
                                         .collect();

    let token: [u8; 8] = core::iter::repeat(0).take((8 - self.tkl) as _)
                                              .chain(core::iter::repeat(1u8).take(self.tkl as _))
                                              .collect::<arrayvec::ArrayVec<_, 8>>()
                                              .into_inner()
                                              .unwrap();

    Message { id: Id(1),
              ty: Type(0),
              ver: Version(0),
              tkl: TokenLength(self.tkl),
              token: Token(u64::from_be_bytes(token)),
              code: Code { class: 2, detail: 5 },
              opts,
              payload: Payload(core::iter::repeat(1u8).take(self.payload_size).collect()) }
  }
}

fn message_to_bytes(c: &mut Criterion) {
  let mut group = c.benchmark_group("kwap_msg/to_bytes");
  group.confidence_level(0.98);
  group.measurement_time(std::time::Duration::from_secs(10));

  let f = |bi: &BenchInput| -> Vec<u8> { Into::<Message>::into(bi).into() };
  let inputs = vec![BenchInput { tkl: 0,
                                 n_opts: 0,
                                 opt_size: 0,
                                 payload_size: 0 },
                    BenchInput { tkl: 4,
                                 n_opts: 4,
                                 opt_size: 8,
                                 payload_size: 16 },
                    BenchInput { tkl: 4,
                                 n_opts: 4,
                                 opt_size: 16,
                                 payload_size: 16 },
                    BenchInput { tkl: 4,
                                 n_opts: 8,
                                 opt_size: 32,
                                 payload_size: 16 },
                    BenchInput { tkl: 8,
                                 n_opts: 8,
                                 opt_size: 64,
                                 payload_size: 16 },
                    BenchInput { tkl: 8,
                                 n_opts: 8,
                                 opt_size: 64,
                                 payload_size: 32 },
                    BenchInput { tkl: 8,
                                 n_opts: 8,
                                 opt_size: 64,
                                 payload_size: 128 },
                    BenchInput { tkl: 8,
                                 n_opts: 16,
                                 opt_size: 64,
                                 payload_size: 128 },
                    BenchInput { tkl: 8,
                                 n_opts: 16,
                                 opt_size: 64,
                                 payload_size: 512 },
                    BenchInput { tkl: 8,
                                 n_opts: 32,
                                 opt_size: 64,
                                 payload_size: 512 },
                    BenchInput { tkl: 8,
                                 n_opts: 32,
                                 opt_size: 64,
                                 payload_size: 2048 },
                    BenchInput { tkl: 8,
                                 n_opts: 32,
                                 opt_size: 256,
                                 payload_size: 2048 },
                    BenchInput { tkl: 8,
                                 n_opts: 32,
                                 opt_size: 512,
                                 payload_size: 2048 },
                    BenchInput { tkl: 8,
                                 n_opts: 32,
                                 opt_size: 512,
                                 payload_size: 4096 },];

  for inp in inputs.iter() {
    group.bench_with_input(BenchmarkId::new("size", f(&inp).len()), &inp, |b, inp| {
           b.iter(|| f(inp))
         });
  }
  group.finish();

  let mut group = c.benchmark_group("coap_lite/to_bytes");
  group.confidence_level(0.98);
  group.measurement_time(std::time::Duration::from_secs(10));

  for inp in inputs.iter() {
    let bytes = f(inp);
    let cl_packet = coap_lite::Packet::from_bytes(&bytes).unwrap();
    group.bench_with_input(BenchmarkId::new("size", bytes.len()), &cl_packet, |b, inp| {
           b.iter(|| inp.to_bytes())
         });
  }

  group.finish();
}

criterion_group!(benches, message_to_bytes);
criterion_main!(benches);
