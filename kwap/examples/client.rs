use std::net::UdpSocket;
use std::time::Instant;

use kwap::config::Alloc;
use kwap::core::{Core, ToAck};
use kwap::req::Req;

#[path = "./server.rs"]
mod server;

macro_rules! block {
  ($e:expr, on_wait {$on_wait:expr}) => {
    loop {
      #[allow(unreachable_patterns)]
      match $e {
        | Err(nb::Error::Other(e)) =>
        {
          #[allow(unreachable_code)]
          break Err(e)
        },
        | Err(nb::Error::WouldBlock) => {
          $on_wait;
        },
        | Ok(x) => break Ok(x),
      }
    }
  };
}

fn main() {
  server::spawn();
 
  let sock = UdpSocket::bind("127.0.0.1:4870").unwrap();
  println!("bound to 127.0.0.1:4870\n");
  let mut core = Core::<UdpSocket, Alloc>::new(sock);

  ping(&mut core);

  let mut req = Req::<Alloc>::get("127.0.0.1", 5683, "hello");

  get_hello(&mut core, req.clone());
  req.non();
  get_hello(&mut core, req);

  server::shutdown();
}

fn ping(core: &mut Core<UdpSocket, Alloc>) {
  println!("pinging coap://localhost:5683");
  let pre_ping = Instant::now();
  let (id, addr) = core.ping("127.0.0.1", 5683).unwrap();
  block!(core.poll_ping(id, &addr), on_wait {
    if (Instant::now() - pre_ping).as_secs() > 5 {
      panic!("ping timed out");
    }
  }).unwrap();
  println!("ping ok! took {}ms", (Instant::now() - pre_ping).as_millis());
  println!();
}

fn get_hello(core: &mut Core<UdpSocket, Alloc>, req: Req<Alloc>) {
  let (id, addr) = core.send_req(req).unwrap();
  println!("GET 127.0.0.1:5683/hello");

  let resp = block!(core.poll_resp(id, &addr), on_wait {()});

  match resp {
    | Ok(rep) => {
      println!("{} {:?}", rep.code().to_string(), rep.payload_string().unwrap());
      println!();
    },
    | Err(e) => {
      eprintln!("error! {:#?}", e);
    },
  }
}
