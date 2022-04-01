use kwap::blocking::client::ClientResultExt;
use kwap::blocking::Client;
use kwap::config::Std;
use kwap::core::Error;
use kwap::req::Req;
use kwap::resp::Resp;

#[path = "./server.rs"]
mod server;

trait Log {
  fn log(self);
}

impl Log for Result<Resp<Std>, kwap::core::Error<Std>> {
  fn log(self) {
    match self {
      | Ok(rep) => {
        println!("client: ok! {} {:?}",
                 rep.code().to_string(),
                 rep.payload_string().unwrap());
        println!();
      },
      | Err(e) => {
        eprintln!("client: error! {:#?}", e);
      },
    }
  }
}

impl Log for Result<Option<Resp<Std>>, Error<Std>> {
  fn log(self) {
    match self {
      | Ok(None) => {
        println!("client: ok! did not receive a response");
        println!();
      },
      | Ok(Some(rep)) => {
        println!("client: ok! {} {:?}",
                 rep.code().to_string(),
                 rep.payload_string().unwrap());
        println!();
      },
      | Err(e) => {
        eprintln!("client: error! {:#?}", e);
      },
    }
  }
}

fn main() {
  server::spawn();

  let mut client = Client::new_std();

  println!("client: PING");
  client.ping("127.0.0.1", 5683)
        .map(|_| println!("client: pinged ok!\n"))
        .unwrap();

  println!("client: CON GET /hello");
  let req = Req::get("127.0.0.1", 5683, "hello");
  client.send(req).log();

  println!("client: NON GET /hello");
  let mut req = Req::get("127.0.0.1", 5683, "hello");
  req.non();
  client.send(req).log();

  println!("client: NON GET /black_hole");
  let mut req = Req::get("127.0.0.1", 5683, "black_hole");
  req.non();
  client.send(req).timeout_ok().log();

  println!("client: NON GET /dropped");
  let req = Req::get("127.0.0.1", 5683, "dropped");
  client.send(req).log();

  server::shutdown();
}
