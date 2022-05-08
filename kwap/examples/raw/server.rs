use std::net::UdpSocket;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::JoinHandle;

use kwap::platform::{self, Std};
use kwap::req::{Method, Req};
use kwap::resp::{code, Resp};
use kwap_msg::{TryFromBytes, TryIntoBytes, Type};

static mut SHUTDOWN: Option<(Sender<()>, Receiver<()>)> = None;

pub fn shutdown() {
  unsafe {
    SHUTDOWN.as_ref().unwrap().0.send(()).unwrap();
  }
}

fn should_shutdown() -> bool {
  unsafe { SHUTDOWN.as_ref().unwrap().1.try_recv().is_ok() }
}

pub fn spawn() -> JoinHandle<()> {
  std::thread::Builder::new().stack_size(32 * 1024 * 1024)
                             .spawn(|| {
                               let p = std::panic::catch_unwind(|| {
                                 server_main();
                               });

                               if p.is_err() {
                                 eprintln!("server panicked! {:?}", p);
                               }
                             })
                             .unwrap()
}

fn server_main() {
  unsafe {
    SHUTDOWN = Some(channel());
  }

  let sock = UdpSocket::bind("0.0.0.0:5683").unwrap();
  sock.set_nonblocking(true).unwrap();
  let mut buf = [0u8; 1152];

  println!("server: up");

  let mut dropped_req_ct = 0u8;

  loop {
    if should_shutdown() {
      println!("server: shutting down...");
      break;
    }

    match sock.recv_from(&mut buf) {
      | Ok((n, addr)) => {
        let msg = platform::Message::<Std>::try_from_bytes(buf.iter().copied().take(n)).unwrap();
        let req = Req::<Std>::from(msg);
        let path = req.get_option(11)
                      .as_ref()
                      .map(|o| &o.value.0)
                      .map(|b| std::str::from_utf8(b).unwrap());

        println!("server: got {:?} {} {} {} bytes",
                 req.msg_type(),
                 req.method(),
                 path.unwrap_or("/"),
                 req.payload_str().unwrap().len());

        let mut resp = Resp::<Std>::for_request(req.clone());
        let send = |r: Resp<Std>| sock.send_to(&r.try_into_bytes::<Vec<u8>>().unwrap(), addr).unwrap();

        match (req.msg_type(), req.method(), path) {
          | (Type::Non, Method::GET, Some("black_hole")) => {
            ();
          },
          | (Type::Con, Method::GET, Some("dropped")) => {
            dropped_req_ct += 1;
            if dropped_req_ct >= 3 {
              resp.set_payload("sorry it took me a bit to respond...".bytes());
              resp.set_code(code::CONTENT);
              send(resp);
            }
          },
          | (_, Method::GET, Some("hello")) => {
            resp.set_payload("hello, world!".bytes());
            resp.set_code(code::CONTENT);
            send(resp);
          },
          // ping
          | (Type::Con, Method::EMPTY, None) if req.payload().is_empty() => {
            resp.set_code(kwap_msg::Code::new(0, 0));
            let mut msg = platform::Message::<Std>::from(resp);
            msg.ty = kwap_msg::Type::Reset;
            sock.send_to(&msg.try_into_bytes::<Vec<u8>>().unwrap(), addr).unwrap();
          },
          | _ => {
            resp.set_code(code::NOT_FOUND);
            send(resp);
          },
        }
      },
      | Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
      | Err(e) => panic!("{:?}", e),
    }
  }
}

#[allow(dead_code)]
fn main() {
  spawn().join().unwrap();
}
