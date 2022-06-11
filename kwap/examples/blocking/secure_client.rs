use std::net::UdpSocket;

use kwap::blocking::client::{ClientResultExt, ClientConfig};
use kwap::blocking::Client;
use kwap::core::Error;
use kwap::net::Addrd;
use kwap::platform::{Std, StdSecure};
use kwap::req::Req;
use kwap::resp::Resp;
use kwap::std::Clock;
use kwap::std::secure::SecureUdpSocket;
use openssl::ssl::{SslConnector, SslVerifyMode, SslMethod};

#[path = "./secure_server.rs"]
mod server;

trait Log {
  fn log(self);
}

impl Log for Result<Resp<StdSecure>, kwap::core::Error<StdSecure>> {
  fn log(self) {
    match self {
      | Ok(rep) => {
        log::info!("ok! {} {:?}",
                   rep.code().to_string(),
                   rep.payload_string().unwrap());
      },
      | Err(e) => {
        log::error!("error! {:?}", e);
      },
    }
  }
}

impl Log for Result<Option<Resp<StdSecure>>, Error<StdSecure>> {
  fn log(self) {
    match self {
      | Ok(None) => {
        log::info!("ok! did not receive a response");
      },
      | Ok(Some(rep)) => {
        log::info!("ok! {} {:?}",
                   rep.code().to_string(),
                   rep.payload_string().unwrap());
      },
      | Err(e) => {
        log::error!("error! {:?}", e);
      },
    }
  }
}

fn main() {
  // simple_logger::init_with_level(log::Level::Trace).unwrap();
  simple_logger::init_with_level(log::Level::Info).unwrap();

  let server = server::spawn();

  let mut conn = SslConnector::builder(SslMethod::dtls()).unwrap();
  conn.set_verify(SslVerifyMode::NONE); // Server uses self-signed cert
  let conn = conn.build();

  let sock = UdpSocket::bind("0.0.0.0:2222").unwrap();
  let mut client = Client::<StdSecure>::new(ClientConfig { clock: Clock::new(), sock: SecureUdpSocket::new_client(conn, sock) });
  let Addrd(_, addr) =
    Client::<Std>::listen_multicast(Clock::new(), server::DISCOVERY_PORT).unwrap();

  log::info!("Got multicast message from {:?}", addr);
  log::info!("Server's location is {:?}", addr);

  log::info!("PING");
  client.ping(format!("{}", addr.ip()), addr.port())
        .map(|_| log::info!("pinged ok!"))
        .unwrap();

  log::info!("CON GET /hello");
  let req = Req::get(addr, "hello");
  client.send(req).log();

  log::info!("NON GET /hello");
  let mut req = Req::get(addr, "hello");
  req.non();
  client.send(req).log();

  log::info!("NON GET /black_hole");
  let mut req = Req::get(addr, "black_hole");
  req.non();
  client.send(req).timeout_ok().log();

  log::info!("NON GET /dropped");
  let req = Req::get(addr, "dropped");
  client.send(req).log();

  let req = Req::get(addr, "dropped");
  client.send(req).log();

  log::info!("CON GET /exit");
  let req = Req::post(addr, "exit");
  client.send(req).log();

  server.join().unwrap();
}
