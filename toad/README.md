[![crates.io](https://img.shields.io/crates/v/kwap.svg)](https://crates.io/crates/kwap)
[![docs.rs](https://docs.rs/kwap/badge.svg)](https://docs.rs/kwap/latest)
![Maintenance](https://img.shields.io/badge/maintenance-activly--developed-brightgreen.svg)

# kwap

`kwap` is a Rust CoAP implementation that aims to be:
- Platform-independent
- Extensible
- Approachable

### CoAP
CoAP is an application-level network protocol that copies the semantics of HTTP
to an environment conducive to **constrained** devices. (weak hardware, small battery capacity, etc.)

This means that you can write and run two-way RESTful communication
between devices very similarly to the networking semantics you are
most likely very familiar with.

#### Similarities to HTTP
CoAP has the same verbs and many of the same semantics as HTTP;
- GET, POST, PUT, DELETE
- Headers (renamed to [Options](https://datatracker.ietf.org/doc/html/rfc7252#section-5.10))
- Data format independent (via the [Content-Format](https://datatracker.ietf.org/doc/html/rfc7252#section-12.3) Option)
- [Response status codes](https://datatracker.ietf.org/doc/html/rfc7252#section-5.9)

#### Differences from HTTP
- CoAP customarily sits on top of UDP (however the standard is [in the process of being adapted](https://tools.ietf.org/id/draft-ietf-core-coap-tcp-tls-11.html) to also run on TCP, like HTTP)
- Because UDP is a "connectionless" protocol, it offers no guarantee of "conversation" between traditional client and server roles. All the UDP transport layer gives you is a method to listen for messages thrown at you, and to throw messages at someone. Owing to this, CoAP machines are expected to perform both client and server roles (or more accurately, _sender_ and _receiver_ roles)
- While _classes_ of status codes are the same (Success 2xx -> 2.xx, Client error 4xx -> 4.xx, Server error 5xx -> 5.xx), the semantics of the individual response codes differ.

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
