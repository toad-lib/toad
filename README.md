ToAD is a CoAP implementation that strives to power client- and server-side CoAP in any language & any environment.

## Project Goals
 - make coap accessible & approachable to those unfamiliar
 - headless CoAP core that can be used by frontends in any language (via JNI/C ABI/WASM)
 - support multi-role M2M communication (coap Endpoints must be able to act as both client & server)
 - make `async`, `alloc` & `std` _completely opt-in_ for clients & servers

## CoAP
CoAP is an application-level network protocol that copies the semantics of HTTP
to an environment conducive to **constrained** devices. (weak hardware, small battery capacity, etc.)

This means that you can write and run two-way RESTful communication
between devices very similarly to the networking semantics you are
most likely very familiar with.

### Similarities to HTTP
CoAP has the same verbs and many of the same semantics as HTTP;
- GET, POST, PUT, DELETE
- Headers (renamed to [Options](https://datatracker.ietf.org/doc/html/rfc7252#section-5.10))
- Data format independent (via the [Content-Format](https://datatracker.ietf.org/doc/html/rfc7252#section-12.3) Option)
- [Response status codes](https://datatracker.ietf.org/doc/html/rfc7252#section-5.9)

### Differences from HTTP
- CoAP customarily sits on top of UDP (however the standard is [in the process of being adapted](https://tools.ietf.org/id/draft-ietf-core-coap-tcp-tls-11.html) to also run on TCP, like HTTP)
- Because UDP is a "connectionless" protocol, it offers no guarantee of "conversation" between traditional client and server roles. All the UDP transport layer gives you is a method to listen for messages thrown at you, and to throw messages at someone. Owing to this, CoAP machines are expected to perform both client and server roles (or more accurately, _sender_ and _receiver_ roles)
- While _classes_ of status codes are the same (Success 2xx -> 2.xx, Client error 4xx -> 4.xx, Server error 5xx -> 5.xx), the semantics of the individual response codes differ.

## How it works (at the moment)
`toad` contains the core CoAP runtime that drives client & server behavior.

It uses `toad_common::Array` to stay decoupled from specific collection types (this makes `alloc` optional)

It uses `nb` to represent nonblocking async io (this will make `async` optional)

It represents the flow of messages through the system as a state machine, allowing for an open-ended system for customizing runtime behavior (this allows for writing idiomatic interfaces in other languages, e.g. invoking JS callbacks on request receipt)

#### Server flow
<details>
  <summary>Click to expand</summary>

```
RecvDgram
    |
 {parse}--------------------
    |                       |
    v                       v
 Recv{Ack,Empty,Request}  MsgParseErr
     |                      |
 {process}--------          |
     |            | <-------
     |      ----> |
     v     |      v
  MsgProcessErr  ToSend
                  |
               {send}
                  |<----------------------
                  |------                 |
                  |      |                |
                  v      v                |
                Done    SendErr --{retry}-
                                          |
                                          |
                                          v
                                     SendPoisoned
```
</details>

#### What a high-level rust interface may look like
<details>
<summary>Click to expand</summary>

```rust
fn main() {
  let udp: toad::Sock = std::UdpSocket::bind(/* addr */).unwrap();
  let server = toad::Server::new(sock).resource(Hello);

  server.start();
}

struct Hello;
impl toad::Resource for Hello {
  const ID: toad::ResourceId = toad::ResourceId::from_str("Hello");

  fn should_handle(&self, req: toad::Req) -> bool {
    req.path.get(0) == Some("hello")
  }

  fn handle(&self, server: &toad::Server, req: toad::Req) -> toad::Result<toad::Rep> {
    if !req.method.is_get() {
      return toad::rep::error::method_not_allowed();
    }

    let name = req.get(1).unwrap_or("World");

    if name == "Jeff" {
      return toad::rep::error::unauthorized("Jeff, I told you this isn't for you. Please leave.");
    }

    let payload = serde_json::json!({"msg": format!("Hello, {}", name)});

    toad::rep::ok::content(payload)
  }
}
```
</details>
