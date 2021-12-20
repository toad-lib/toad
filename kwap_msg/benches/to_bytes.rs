use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
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
    let opts: Vec<Opt> = (0..self.n_opts).map(|n| {
      Opt {
        delta: OptDelta(n as _),
        value: OptValue(core::iter::repeat(1).take(self.opt_size).collect()),
      }
    }).collect();

    let token: [u8; 8] = core::iter::repeat(0)
        .take((8 - self.tkl) as _)
        .chain(core::iter::repeat(1u8).take(self.tkl as _))
        .collect::<arrayvec::ArrayVec<_, 8>>().into_inner().unwrap();

    Message {
      id: Id(1),
      ty: Type(0),
      ver: Version(0),
      tkl: TokenLength(self.tkl),
      token: Token(u64::from_be_bytes(token)),
      code: Code {class: 2, detail: 5},
      opts,
      payload: Payload(core::iter::repeat(1u8).take(self.payload_size).collect()),
    }
  }
}

fn message_to_bytes(c: &mut Criterion) {
  let mut group = c.benchmark_group("message_to_bytes");
  group.warm_up_time(std::time::Duration::from_millis(100));
  group.measurement_time(std::time::Duration::from_secs(1));

  let f = |bi: &BenchInput| -> Vec<u8> { Into::<Message>::into(bi).into() };

  for n_opts in [1usize, 2, 4, 8, 16] {
      let inp = BenchInput {tkl: 0, n_opts, opt_size: 16, payload_size: 0};
      group.bench_with_input(BenchmarkId::new("n_opts", n_opts), &inp, |b, inp| b.iter(|| f(inp)));
  }

  for opt_size in [1usize, 8, 64] {
      let inp = BenchInput {tkl: 0, n_opts: 1, opt_size, payload_size: 0};
      group.bench_with_input(BenchmarkId::new("opt_size", opt_size), &inp, |b, inp| b.iter(|| f(inp)));
  }

  for payload_size in [1usize, 8, 32, 64, 128, 256, 512, 1024] {
      let inp = BenchInput {tkl: 0, n_opts: 0, opt_size: 0, payload_size};
      group.bench_with_input(BenchmarkId::new("payload_size", payload_size), &inp, |b, inp| b.iter(|| f(inp)));
  }

  for tkl in 0..=8 {
      let inp = BenchInput {tkl, n_opts: 0, opt_size: 0, payload_size: 0};
      group.bench_with_input(BenchmarkId::new("tkl", tkl), &inp, |b, inp| b.iter(|| f(inp)));
  }

  group.finish();
}

criterion_group!(benches, message_to_bytes);
criterion_main!(benches);
