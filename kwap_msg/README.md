[![crates.io](https://img.shields.io/crates/v/{{PACKAGE}}.svg)](https://crates.io/crates/{{PACKAGE}})
[![docs.rs](https://docs.rs/{{PACKAGE}}/badge.svg)](https://docs.rs/{{PACKAGE}}/latest)
![Maintenance](https://img.shields.io/badge/maintenance-activly--developed-brightgreen.svg)

# kwap-msg

Low-level representation of CoAP messages.

Performs comparably&#42; to the `Packet` structure in [`coap_lite`](https://github.com/martindisch/coap-lite)

&#42; _(benchmark data available for [this library](./criterion/reports/kwap_msg_to_bytes/index.html) as well as [coap_lite](./criterion/reports/coap_lite_to_bytes/index.html))_

If you're a library user, you probably want `req`/`resp` instead!

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
