use std::fs::File;
use std::io::Read;
use std::net::UdpSocket;
use std::thread::JoinHandle;

use toad::blocking::server::{Action, Actions};
use toad::net::Addrd;
use toad::platform::StdSecure;
use toad::req::Req;
use toad::resp::{code, Resp};
use toad::std::{Clock, SecureUdpSocket};
use openssl::pkey::PKey;
use openssl::rsa::Rsa;
use openssl::x509::X509;

const PORT: u16 = 1111;
pub const DISCOVERY_PORT: u16 = 1234;

mod service {
  use std::time::{Duration, Instant};

  use toad::req::Method;
  use Action::{Continue, Exit, Insecure, SendReq, SendResp};

  use super::*;
  static mut BROADCAST_RECIEVED: bool = false;
  static mut LAST_BROADCAST: Option<Instant> = None;

  /// CON/NON POST /exit
  pub fn exit(req: &Addrd<Req<StdSecure>>) -> Actions<StdSecure> {
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
  pub fn say_hello(req: &Addrd<Req<StdSecure>>) -> Actions<StdSecure> {
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
  pub fn not_found(req: &Addrd<Req<StdSecure>>) -> Actions<StdSecure> {
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
  pub fn close_multicast_broadcast(_: &Addrd<Req<StdSecure>>) -> Actions<StdSecure> {
    unsafe {
      BROADCAST_RECIEVED = true;
      log::trace!("No longer sending broadcasts");
    }

    Continue.into()
  }

  /// If we haven't received a request yet,
  /// send an empty NON request to the all_coap_devices
  /// multicast address on port `DISCOVERY_PORT`
  pub fn send_multicast_broadcast() -> Actions<StdSecure> {
    match unsafe { BROADCAST_RECIEVED } {
      | false
        if unsafe {
          LAST_BROADCAST.map(|inst| inst < Instant::now() - Duration::from_secs(5))
                        .unwrap_or(true)
        } =>
      {
        unsafe {
          LAST_BROADCAST = Some(Instant::now());
        }
        let addr = toad::multicast::all_coap_devices(DISCOVERY_PORT);

        let mut req = Req::<StdSecure>::post(addr, "");
        req.non();

        Insecure(SendReq(Addrd(req, addr)).into()).then(Continue)
      },
      | _ => Actions::just(Continue),
    }
  }
}

pub fn spawn() -> JoinHandle<()> {
  std::thread::Builder::new().stack_size(32 * 1024 * 1024)
                             .spawn(|| {
                               let (mut pkey_file, mut cert_file) = (vec![], vec![]);
                               File::open("toad/examples/key.pem").unwrap().read_to_end(&mut pkey_file).unwrap();
                               File::open("toad/examples/cert.pem").unwrap().read_to_end(&mut cert_file).unwrap();

                               let pkey = PKey::from_rsa(Rsa::private_key_from_pem(&pkey_file).unwrap()).unwrap();
                               let cert = X509::from_pem(&cert_file).unwrap();

                               let sock = UdpSocket::bind(&format!("0.0.0.0:{}", PORT)).unwrap();
                               let sock = SecureUdpSocket::try_new_server(sock, pkey, cert).unwrap();

                               let mut server =
                                 toad::blocking::Server::<StdSecure, Vec<_>>::new(sock, Clock::new());

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
  simple_logger::init_with_level(log::Level::Trace).unwrap();

  spawn().join().unwrap();
}
