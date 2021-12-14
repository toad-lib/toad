# coapr8 (pr. "cooperate")
An extensible multi-platform rusty implementation of a CoAP protocol.

## Project goals
 - support multi-role M2M communication (nodes can act as both client & server)
 - optional async runtime support
 - optional alloc / no-alloc
 - no_std

```rust
use coapr8::filter;

fn main() {
  let udp: coapr8::Sock = std::UdpSocket::bind(/* addr */).unwrap();
  let server = coapr8::Server::new(sock)
                              .resource(Hello);

  server.start();
}

struct Hello;
impl coapr8::Resource for Hello {
  pub fn should_handle(req: coapr8::Req) -> bool {
    req.path.get(0) == Some("hello")
  }

  pub fn handle(req: coapr8::Req) -> coapr8::Result<coapr8::Rep> {
    if !req.method.is_get() {
      return coapr8::rep::error::method_not_allowed();
    }

    let name = req.get(1).unwrap_or("World");

    if name == "Jeff" {
      return coapr8::rep::error::unauthorized("Jeff, I told you this isn't for you. Please leave.");
    }

    let payload = serde_json::json!({"msg": format!("Hello, {}", name)});

    coapr8::rep::ok::content(payload)
  }
}
```
