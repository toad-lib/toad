use std::net::UdpSocket;
use std::time::Instant;

use kwap::config::Alloc;
use kwap::core::Core;
use kwap::req::Req;

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
  let sock = UdpSocket::bind("0.0.0.0:4870").unwrap();
  println!("bound to 0.0.0.0:4870\n");
  let mut core = Core::<UdpSocket, Alloc>::new(sock);

  ping(&mut core);

  let req = Req::<Alloc>::get("0.0.0.0", 5683, "hello");

  get_hello(&mut core, req.clone());
  get_hello(&mut core, req);
}

fn ping(core: &mut Core<UdpSocket, Alloc>) {
  println!("pinging coap://localhost:5683");
  let pre_ping = Instant::now();
  let ping_id = core.ping("0.0.0.0", 5683).unwrap();
  block!(core.poll_ping(ping_id), on_wait {
    if (Instant::now() - pre_ping).as_secs() > 30 {
      panic!("ping timed out");
    }
  }).unwrap();
  println!("ping ok! took {}ms", (Instant::now() - pre_ping).as_millis());
  println!();
}

fn get_hello(core: &mut Core<UdpSocket, Alloc>, req: Req<Alloc>) {
  let id = req.msg_id();
  core.send_req(req).unwrap();
  println!("GET 0.0.0.0:5683/hello");

  let resp = block!(core.poll_resp(id), on_wait {()});

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
