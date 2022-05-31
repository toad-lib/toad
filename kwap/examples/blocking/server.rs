use std::thread::JoinHandle;

use kwap::blocking::server::{Action, Continue};
use kwap::net::{Addrd, Socket};
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
      log::info!("a client said exit");
      (Continue::No, Action::Exit)
    },
    | _ => (Continue::Yes, Action::Nop),
  }
}

fn say_hello(req: &Addrd<Req<Std>>) -> (Continue, Action<Std>) {
  match req.data().path().unwrap() {
    | Some("hello") => {
      log::info!("a client said hello");
      let resp = req.as_ref()
                    .map(Resp::for_request)
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
  log::info!("not found");
  let resp = req.as_ref()
                .map(Resp::for_request)
                .map(Option::unwrap)
                .map(|mut resp| {
                  resp.set_code(code::NOT_FOUND);
                  resp
                });
  (Continue::No, Action::Send(resp.map(Into::into)))
}

fn log_msg(msg: &Addrd<platform::Message<Std>>) {
  log::info!(
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

fn close_multicast_broadcast(_: &Addrd<Req<Std>>) -> (Continue, Action<Std>) {
  unsafe {
    BROADCAST_RECIEVED = true;
  }
  (Continue::Yes, Action::Nop)
}

fn log(req: &Addrd<Req<Std>>) -> (Continue, Action<Std>) {
  unsafe {
    BROADCAST_RECIEVED = true;
  }
  log::info!("recv:");
  log_msg(&req.clone().map(Into::into));
  (Continue::Yes, Action::Nop)
}

static mut BROADCAST_RECIEVED: bool = false;

fn on_tick() -> Action<Std> {
  let received = unsafe { BROADCAST_RECIEVED };
  if !received {
    let addr = kwap::multicast::all_coap_devices(1234);
    let mut req = Req::<Std>::post(addr, "");
    req.non();
    req.set_payload(
                    r#"hi!
i'm a CoAP server named Barry! :)
This message has been sent to the "All CoAP Devices" multicast address.

Please reach out to me directly to learn about what I can do!"#,
    );
    Action::SendReq(Addrd(req.into(), addr))
  } else {
    Action::Exit
  }
}

pub fn spawn() -> JoinHandle<()> {
  std::thread::Builder::new().stack_size(32 * 1024 * 1024)
                             .spawn(|| {
                               let sock = <std::net::UdpSocket as Socket>::bind(kwap::multicast::all_coap_devices(5634)).unwrap();
                               // sock.join_multicast(kwap::multicast::ALL_COAP_DEVICES_ADDR.into()).unwrap();

                               let mut server =
                                 kwap::blocking::Server::<Std, Vec<_>>::new(sock, kwap::std::Clock::new());
                               let out = server.start_tick(Some(&on_tick));

                               if out.is_err() {
                                 log::error!("panicked! {:?}", out);
                               } else {
                                 log::info!("multicast broadcaster closing");
                               }
                             }).ok();

  std::thread::Builder::new().stack_size(32 * 1024 * 1024)
                             .spawn(|| {
                               let mut server =
                                 kwap::blocking::Server::try_new([192, 168, 0, 45], 5634).unwrap();

                               server.middleware(&close_multicast_broadcast);
                               server.middleware(&log);
                               server.middleware(&exit_respond);
                               server.middleware(&exit);
                               server.middleware(&say_hello);
                               server.middleware(&not_found);
                               let out = server.start();

                               if out.is_err() {
                                 log::error!("panicked! {:?}", out);
                               }
                             })
                             .unwrap()
}

#[allow(dead_code)]
fn main() {
  spawn().join().unwrap();
}
