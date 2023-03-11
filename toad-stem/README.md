[![crates.io](https://img.shields.io/crates/v/toad-stem.svg)](https://crates.io/crates/toad-stem)
[![docs.rs](https://docs.rs/toad-stem/badge.svg)](https://docs.rs/toad-stem/latest)
![Maintenance](https://img.shields.io/badge/maintenance-activly--developed-brightgreen.svg)

# toad-stem

This microcrate provides a mutable memory wrapper that is thread-safe
and usable on `no_std` platforms by using [`std::sync::RwLock`]
when crate feature `std` is enabled (this is the default) and
falling back to [`core::cell::Cell`] when `std` disabled.

the API of the core struct [`Stem`] was chosen to discourage long-lived
immutable references to the cell's contents, so that deadlocks are less likely.

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
