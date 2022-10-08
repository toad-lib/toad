use criterion::{criterion_group, criterion_main, BatchSize, Criterion};

// Benchmarking various heapless/no-alloc collections alongside std's Vec and arrays
// Results observed: heapless is slower than arrayvec is slower than tinyvec (esp. `chain`)

fn collections(c: &mut Criterion) {
  c.bench_function("array::alloc", |b| b.iter(|| [0u8; 2048]));
  c.bench_function("alloc::vec::Vec::alloc", |b| {
     b.iter(|| std::vec::Vec::<u8>::with_capacity(2048))
   });
  c.bench_function("heapless::Vec::alloc", |b| {
     b.iter(heapless::Vec::<u8, 2048>::new)
   });
  c.bench_function("arrayvec::ArrayVec::alloc", |b| {
     b.iter(arrayvec::ArrayVec::<u8, 2048>::new)
   });
  c.bench_function("tinyvec::ArrayVec::alloc", |b| {
     b.iter(tinyvec::ArrayVec::<[u8; 2048]>::new)
   });
  c.bench_function("std::vec::Vec::push", |b| {
     b.iter_batched(|| std::vec::Vec::<u8>::with_capacity(16),
                    |mut vec| vec.push(255),
                    BatchSize::SmallInput)
   });
  c.bench_function("heapless::Vec::push", |b| {
     b.iter_batched(heapless::Vec::<u8, 16>::new,
                    |mut vec| vec.push(255),
                    BatchSize::SmallInput)
   });
  c.bench_function("arrayvec::ArrayVec::push", |b| {
     b.iter_batched(arrayvec::ArrayVec::<u8, 16>::new,
                    |mut vec| vec.push(255),
                    BatchSize::SmallInput)
   });
  c.bench_function("tinyvec::ArrayVec::push", |b| {
     b.iter_batched(tinyvec::ArrayVec::<[u8; 16]>::new,
                    |mut vec| vec.push(255),
                    BatchSize::SmallInput)
   });
  c.bench_function("std::vec::Vec::extend", |b| {
     b.iter_batched(|| {
                      (std::vec::Vec::<u8>::with_capacity(2048), core::iter::repeat(255).take(2048))
                    },
                    |(mut vec, other)| vec.extend(other),
                    BatchSize::SmallInput)
   });
  c.bench_function("heapless::Vec::extend", |b| {
     b.iter_batched(|| (heapless::Vec::<u8, 2048>::new(), core::iter::repeat(255).take(2048)),
                    |(mut vec, other)| vec.extend(other),
                    BatchSize::SmallInput)
   });
  c.bench_function("arrayvec::ArrayVec::extend", |b| {
     b.iter_batched(|| (arrayvec::ArrayVec::<u8, 2048>::new(), core::iter::repeat(255).take(2048)),
                    |(mut vec, other)| vec.extend(other),
                    BatchSize::SmallInput)
   });
  c.bench_function("tinyvec::ArrayVec::extend", |b| {
     b.iter_batched(|| (tinyvec::ArrayVec::<[u8; 2048]>::new(), core::iter::repeat(255).take(2048)),
                    |(mut vec, other)| vec.extend(other),
                    BatchSize::SmallInput)
   });
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(100).warm_up_time(std::time::Duration::from_secs(5))
           .measurement_time(std::time::Duration::from_secs(15));
    targets = collections
}
criterion_main!(benches);
