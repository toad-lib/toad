[![crates.io](https://img.shields.io/crates/v/kwap-msg.svg)](https://crates.io/crates/kwap-msg)
[![docs.rs](https://docs.rs/kwap-msg/badge.svg)](https://docs.rs/kwap-msg/latest)
![Maintenance](https://img.shields.io/badge/maintenance-activly--developed-brightgreen.svg)

# kwap-msg

## kwap_msg
Low-level representation of CoAP messages.

### `alloc` vs `no_alloc`
kwap_msg implements CoAP messages as either backed by:
- `alloc`: dynamically growable heap-allocated buffers
- `no_alloc`: static stack-allocated buffers

`alloc::Message` can be much easier to use and performs comparably to `no_alloc`, however it does require:
`std` or [a global allocator](https://doc.rust-lang.org/std/alloc/index.html)

### Performance
This crate uses `criterion` to measure performance of the heaped & heapless implementations in this crate as well as `coap_lite::Packet`.

In general, `kwap_msg::alloc::Message` is faster than coap_lite, which is much faster than `no_alloc::Message`.

Benchmarks:
#### Serializing to bytes
![chart](https://raw.githubusercontent.com/clov-coffee/kwap/main/kwap_msg/docs/from_bytes.svg)

#### Deserializing from bytes
![chart](https://raw.githubusercontent.com/clov-coffee/kwap/main/kwap_msg/docs/to_bytes.svg)

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
