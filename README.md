<img src="https://raw.githubusercontent.com/clov-coffee/toad/main/static/banner.png" />

A CoAP implementation that strives to power client- and server-side CoAP in any language & any environment.

# CoAP?
CoAP is an application-level network protocol that copies the semantics of HTTP
to an environment conducive to **constrained** devices. (weak hardware, small battery capacity, etc.)

This means that you can write and run two-way RESTful communication
between devices very similarly to the networking semantics you are
most likely very familiar with.

## CoAP vs HTTP
CoAP provides a high performance + low latency alternative to HTTP that uses much of the same terminology and semantics.

In CoAP, you'll find familiar things like verbs (_GET, POST, PUT, DELETE_), headers (_aka. Options_) and status code (_4.04 NOT FOUND_)
but you also have access to some extra levers that let you customize behavior of requests and responses, such as "I don't need to know that you received this message."

# The library
## Contributing
* There are [examples](https://github.com/toad-lib/toad/tree/main/toad/examples) provided in each library that demo some high (or low) level use-cases that this library aims to cover
* The [issues](https://github.com/toad-lib/toad/issues) are a good place to start if you're interested in contributing directly. At this stage, the project issues are a living backlog of work-to-be-done before stabilizing and promoting the repo.
* You (contributor) should just make sure tests pass (`cargo make test`). I'll get the other CI steps to pass to make your contribution experience painless.
* TODO: document architecture at a high level

## Setup
 * install the [rust language](https://rustup.rs/)
 * clone this repo `git clone git@github.com:toad-lib/toad`
 * install cargo-make `cargo install cargo-make`
 * try running an example `cd toad; cargo run --example server`

## How we define success
 - toad is significantly faster and lower latency than comparable HTTP libraries
 - toad answers questions like "what is coap?" and "why should i care?"
 - toad has frontends available in major ecosystems (Android, iOS, Node)
 - toad is fully usable on constrained bare metal devices in both server & client roles
