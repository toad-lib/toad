[![crates.io](https://img.shields.io/crates/v/toad-msg.svg)](https://crates.io/crates/toad-msg)
[![docs.rs](https://docs.rs/toad-msg/badge.svg)](https://docs.rs/toad-msg/latest)
![Maintenance](https://img.shields.io/badge/maintenance-activly--developed-brightgreen.svg)

# toad-msg

Low-level representation of CoAP messages.

The most notable item in `toad_msg` is `Message`;
a CoAP message very close to the actual byte layout.

### Allocation
CoAP messages have some attributes whose size is dynamic:
- The message payload (in http terms: the request/response body)
- the number of options (in http terms: headers)
- the value of an option (in http terms: header value)

`Message` does not require an allocator and has no opinions about what kind of collection
it uses internally to store these values.

It solves this problem by being generic over the collections it needs and uses a `Collection` trait
to capture its idea of what makes a collection useful.

This means that you may use a provided implementation (for `Vec` or `tinyvec::ArrayVec`)
or provide your own collection (see the [custom collections example](https://github.com/clov-coffee/toad/blob/main/toad_msg/examples/custom_collections.rs))

```rust
//! Note: both of these type aliases are exported by `toad_msg` for convenience.

use tinyvec::ArrayVec;
use toad_msg::{Message, Opt};

//                        Message Payload byte buffer
//                        |
//                        |        Option Value byte buffer
//                        |        |
//                        |        |        Collection of options in the message
//                        vvvvvvv  vvvvvvv  vvvvvvvvvvvvvvvvv
type VecMessage = Message<Vec<u8>, Vec<u8>, Vec<Opt<Vec<u8>>>>;

// Used like: `ArrayVecMessage<1024, 256, 16>`; a message that can store a payload up to 1024 bytes, and up to 16 options each with up to a 256 byte value.
type ArrayVecMessage<
       const PAYLOAD_SIZE: usize,
       const OPT_SIZE: usize,
       const NUM_OPTS: usize,
     > = Message<
           ArrayVec<[u8; PAYLOAD_SIZE]>,
           ArrayVec<[u8; OPT_SIZE]>,
           ArrayVec<[Opt<ArrayVec<[u8; OPT_SIZE]>>; NUM_OPTS]>,
         >;
```

It may look a little ugly, but a core goal of `toad` is to be platform- and alloc-agnostic.

### Performance
This crate uses `criterion` to measure performance of the heaped & heapless implementations in this crate as well as `coap_lite::Packet`.

In general, `toad_msg::VecMessage` performs identically to coap_lite (+/- 5%), and both are **much** faster than `toad_msg::ArrayVecMessage`.

Benchmarks:
#### Serializing to bytes
<details><summary>**Click to expand chart**</summary>

![chart](https://raw.githubusercontent.com/clov-coffee/toad/main/toad_msg/docs/from_bytes.svg)
</details>

#### Deserializing from bytes
<details><summary>**Click to expand chart**</summary>

![chart](https://raw.githubusercontent.com/clov-coffee/toad/main/toad_msg/docs/to_bytes.svg)
</details>

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
