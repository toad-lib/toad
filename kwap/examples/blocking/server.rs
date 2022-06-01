use std::thread::JoinHandle;

use kwap::blocking::server::{Action, Actions};
use kwap::net::{Addrd, Socket};
use kwap::platform::Std;
use kwap::req::Req;
use kwap::resp::{code, Resp};

const PORT: u16 = 5634;
pub const DISCOVERY_PORT: u16 = 1234;

mod service {
  use Action::{Continue, Exit, SendReq, SendResp};

  use super::*;
  static mut BROADCAST_RECIEVED: bool = false;

  pub fn exit(req: &Addrd<Req<Std>>) -> Actions<Std> {
    match req.data().path().unwrap() {
      | Some("exit") => {
        let mut resp = req.as_ref().map(Resp::for_request).map(Option::unwrap);

        resp.0.set_code(code::CONTENT);
        resp.0.set_payload("goodbye, world!".bytes());
        log::info!("a client said exit");
        SendResp(resp).then(Exit)
      },
      | _ => Continue.into(),
    }
  }

  pub fn say_hello(req: &Addrd<Req<Std>>) -> Actions<Std> {
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
        SendResp(resp).into()
      },
      | _ => Continue.into(),
    }
  }

  pub fn not_found(req: &Addrd<Req<Std>>) -> Actions<Std> {
    log::info!("not found");
    let resp = req.as_ref()
                  .map(Resp::for_request)
                  .map(Option::unwrap)
                  .map(|mut resp| {
                    resp.set_code(code::NOT_FOUND);
                    resp
                  });

    SendResp(resp).into()
  }

  pub fn close_multicast_broadcast(_: &Addrd<Req<Std>>) -> Actions<Std> {
    unsafe {
      BROADCAST_RECIEVED = true;
    }

    Continue.into()
  }

  pub fn on_tick() -> Actions<Std> {
    let received = unsafe { BROADCAST_RECIEVED };
    if !received {
      let addr = kwap::multicast::all_coap_devices(DISCOVERY_PORT);
      let mut req = Req::<Std>::post(addr, "");
      req.non();
      req.set_payload(PORT);

      SendReq(Addrd(req, addr)).then(Continue)
    } else {
      Exit.into()
    }
  }
}

pub fn spawn() -> JoinHandle<()> {
  std::thread::Builder::new().stack_size(32 * 1024 * 1024)
                             .spawn(|| {
                               let sock = <std::net::UdpSocket as Socket>::bind(kwap::multicast::all_coap_devices(1235)).unwrap();

                               let mut server =
                                 kwap::blocking::Server::<Std, Vec<_>>::new(sock, kwap::std::Clock::new());
                               let out = server.start_tick(Some(&service::on_tick));

                               if out.is_err() {
                                 log::error!("err! {:?}", out);
                               } else {
                                 log::info!("multicast broadcaster closing");
                               }
                             }).ok();

  std::thread::Builder::new().stack_size(32 * 1024 * 1024)
                             .spawn(|| {
                               let mut server =
                                 kwap::blocking::Server::try_new([0, 0, 0, 0], PORT).unwrap();

                               server.middleware(&service::close_multicast_broadcast);
                               server.middleware(&service::exit);
                               server.middleware(&service::say_hello);
                               server.middleware(&service::not_found);

                               let out = server.start();

                               if out.is_err() {
                                 log::error!("err! {:?}", out);
                               }
                             })
                             .unwrap()
}

#[allow(dead_code)]
fn main() {
  spawn().join().unwrap();
}
