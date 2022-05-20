use std::thread::JoinHandle;

use kwap::blocking::server::{Action, Continue};
use kwap::net::Addrd;
use kwap::platform::{self, Std};
use kwap::req::Req;
use kwap::resp::{code, Resp};
use kwap_msg::Type;

fn exit_respond(req: &Addrd<Req<Std>>) -> (Continue, Action<Std>) {
  let Addrd(resp, addr) = req.as_ref().map(|req| match req.msg_type() {
                                        | Type::Con => Some(Resp::ack(req)),
                                        | Type::Non => Some(Resp::con(req)),
                                        | _ => None,
                                      });

  resp.map(|mut resp| {
        resp.set_code(code::CONTENT);
        resp.set_payload("goodbye, world!".bytes());

        match req.data().path().unwrap() {
          | Some("exit") => (Continue::Yes, Action::Send(Addrd(resp.into(), addr))),
          | _ => (Continue::Yes, Action::Nop),
        }
      })
      .unwrap_or((Continue::Yes, Action::Nop))
}

fn exit(req: &Addrd<Req<Std>>) -> (Continue, Action<Std>) {
  match req.data().path().unwrap() {
    | Some("exit") => {
      println!("a client said exit");
      (Continue::No, Action::Exit)
    },
    | _ => (Continue::Yes, Action::Nop),
  }
}

fn say_hello(req: &Addrd<Req<Std>>) -> (Continue, Action<Std>) {
  match req.data().path().unwrap() {
    | Some("hello") => {
      println!("a client said hello");
      let resp = req.as_ref()
                    .map(|req| Resp::for_request(req))
                    .map(Option::unwrap)
                    .map(|mut resp| {
                      resp.set_code(code::CONTENT);
                      resp.set_payload("hello, world!".bytes());
                      resp
                    });
      (Continue::No, Action::Send(resp.map(Into::into)))
    },
    | _ => (Continue::Yes, Action::Nop),
  }
}

fn not_found(req: &Addrd<Req<Std>>) -> (Continue, Action<Std>) {
  println!("not found");
  let resp = req.as_ref()
                .map(|req| Resp::for_request(req))
                .map(Option::unwrap)
                .map(|mut resp| {
                  resp.set_code(code::NOT_FOUND);
                  resp
                });
  (Continue::No, Action::Send(resp.map(Into::into)))
}

fn log_msg(msg: &Addrd<platform::Message<Std>>) {
  println!(
           r#"{{
  id: {:?},
  token: {:?},
  addr: {:?}
}}"#,
           msg.data().id,
           msg.data().token,
           msg.addr()
  );
}

fn log(req: &Addrd<Req<Std>>) -> (Continue, Action<Std>) {
  println!("recv:");
  log_msg(&req.clone().map(Into::into));
  (Continue::Yes, Action::Nop)
}

pub fn spawn() -> JoinHandle<()> {
  std::thread::Builder::new().stack_size(32 * 1024 * 1024)
                             .spawn(|| {
                               let mut server = kwap::blocking::Server::try_new([127, 0, 0, 1], 5683).unwrap();
                               server.middleware(&log);
                               server.middleware(&exit_respond);
                               server.middleware(&exit);
                               server.middleware(&say_hello);
                               server.middleware(&not_found);
                               let out = server.start();

                               if out.is_err() {
                                 eprintln!("server panicked! {:?}", out);
                               }
                             })
                             .unwrap()
}

#[allow(dead_code)]
fn main() {
  spawn().join().unwrap();
}
