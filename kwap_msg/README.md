[![crates.io](https://img.shields.io/crates/v/{{PACKAGE}}.svg)](https://crates.io/crates/{{PACKAGE}})
[![docs.rs](https://docs.rs/{{PACKAGE}}/badge.svg)](https://docs.rs/{{PACKAGE}}/latest)
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

### `kwap_msg::Message` vs `coap_lite::Packet`
Benchmarks are available comparing `kwap_msg::alloc::Message`, `kwap_msg::no_alloc::Message` and `coap_lite::Packet`.

#### Serializing to bytes
![chart](../docs/to_bytes.svg)

#### Deserializing from bytes
![chart](../docs/from_bytes.svg)

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
