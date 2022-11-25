use std::thread::JoinHandle;

use toad::blocking::server::{Action, Actions};
use toad::net::Addrd;
use toad::platform::Std;
use toad::resp::code;
use toad::std::{Req, Resp};

const PORT: u16 = 5555;
pub const DISCOVERY_PORT: u16 = 1234;

mod service {
  use toad::req::Method;
  use Action::{Continue, Exit, SendReq, SendResp};

  use super::*;
  static mut BROADCAST_RECIEVED: bool = false;

  /// CON/NON POST /exit
  pub fn exit(req: &Addrd<Req>) -> Actions<Std> {
    match (req.data().method(), req.data().path().unwrap()) {
      | (Method::POST, Some("exit")) => {
        let mut resp = req.as_ref().map(Resp::for_request).map(Option::unwrap);

        resp.0.set_code(code::CONTENT);
        resp.0.set_payload("goodbye, world!".bytes());
        log::info!("a client said exit");
        SendResp(resp).then(Exit)
      },
      | _ => Continue.into(),
    }
  }

  /// CON/NON GET /hello
  pub fn say_hello(req: &Addrd<Req>) -> Actions<Std> {
    match (req.data().method(), req.data().path().unwrap()) {
      | (Method::GET, Some("hello")) => {
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

  /// If we get here, that means that all other services
  /// failed to process and we should respond 4.04
  pub fn not_found(req: &Addrd<Req>) -> Actions<Std> {
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

  /// Stop sending messages to the multicast address once we receive a request
  /// because that means we've been discovered
  pub fn close_multicast_broadcast(_: &Addrd<Req>) -> Actions<Std> {
    unsafe {
      BROADCAST_RECIEVED = true;
      log::trace!("No longer sending broadcasts");
    }

    Continue.into()
  }

  /// If we haven't received a request yet,
  /// send an empty NON request to the all_coap_devices
  /// multicast address on port `DISCOVERY_PORT`
  pub fn send_multicast_broadcast() -> Actions<Std> {
    match unsafe { BROADCAST_RECIEVED } {
      | true => Actions::just(Continue),
      | false => {
        let addr = toad::multicast::all_coap_devices(DISCOVERY_PORT);

        let mut req = Req::post(addr, "");
        req.non();

        SendReq(Addrd(req, addr)).then(Continue)
      },
    }
  }
}

pub fn spawn() -> JoinHandle<()> {
  std::thread::Builder::new().stack_size(32 * 1024 * 1024)
                             .spawn(|| {
                               let mut server =
                                 toad::blocking::Server::try_new([0, 0, 0, 0], PORT).unwrap();

                               server.middleware(&service::close_multicast_broadcast);
                               server.middleware(&service::exit);
                               server.middleware(&service::say_hello);
                               server.middleware(&service::not_found);

                               let out =
                                 server.start_tick(Some(&service::send_multicast_broadcast));

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
