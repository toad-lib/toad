use criterion::{criterion_group, criterion_main, Criterion};

fn some_vs_once(c: &mut Criterion) {
  c.bench_function("core::iter::once", |b| {
     b.iter(|| std::iter::once(0u8).collect::<Vec<_>>())
   });
  c.bench_function("Option::into_iter", |b| {
     b.iter(|| Some(0u8).into_iter().collect::<Vec<_>>())
   });
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(100).warm_up_time(std::time::Duration::from_secs(5))
           .measurement_time(std::time::Duration::from_secs(15));
    targets = some_vs_once
}
criterion_main!(benches);
