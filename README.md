# kwap
A CoAP implementation that strives to power client- and server-side CoAP in any language & any environment.

## Goals
 - make coap accessible & approachable to those unfamiliar
 - headless CoAP core that can be used by frontends in any language (via JNI/C ABI/WASM)
 - support multi-role M2M communication (coap Endpoints must be able to act as both client & server)
 - optional async support
 - make alloc & std _completely optional_

## Big picture ideas
 - `kwap_msg` for pulling messages off the wire
 - asynchronous event-driven architecture:
   - `fn on(srv: &Server, e: Event, f: fn(&Server, Event) -> Event) -> ()`
   - `Nop`
   - `RecvDgram(Vec<u8>)`
   - `MsgParseErr(kwap::packet::ParseError)`
   - `RecvAck(kwap::msg::Ack)`
   - `RecvEmpty(kwap::msg::Empty)`
   - `RecvRequest(kwap::req::Req)`
   - `GetRespErr(kwap::msg::Msg, kwap::Error)`
   - `ResourceChanged(kwap::ResourceId)`
   - `ToSend(kwap::msg::Msg)`
   - `SendErr(kwap::resp::Resp, kwap::Error)`
   - `SendPoisoned(kwap::resp::Resp)`

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

```rust
fn main() {
  let udp: kwap::Sock = std::UdpSocket::bind(/* addr */).unwrap();
  let server = kwap::Server::new(sock).resource(Hello);

  server.start();
}

struct Hello;
impl kwap::Resource for Hello {
  const ID: kwap::ResourceId = kwap::ResourceId::from_str("Hello");

  fn should_handle(&self, req: kwap::Req) -> bool {
    req.path.get(0) == Some("hello")
  }

  fn handle(&self, server: &kwap::Server, req: kwap::Req) -> kwap::Result<kwap::Rep> {
    if !req.method.is_get() {
      return kwap::rep::error::method_not_allowed();
    }

    let name = req.get(1).unwrap_or("World");

    if name == "Jeff" {
      return kwap::rep::error::unauthorized("Jeff, I told you this isn't for you. Please leave.");
    }

    let payload = serde_json::json!({"msg": format!("Hello, {}", name)});

    kwap::rep::ok::content(payload)
  }
}
```
