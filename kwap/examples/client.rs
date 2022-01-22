use kwap::blocking::Client;
use kwap::config::Std;
use kwap::req::Req;
use kwap::resp::Resp;

#[path = "./server.rs"]
mod server;

trait Log {
  fn log(self);
}

impl Log for kwap::blocking::client::Result<Resp<Std>> {
  fn log(self) {
    match self {
      | Ok(rep) => {
        println!("{} {:?}", rep.code().to_string(), rep.payload_string().unwrap());
        println!();
      },
      | Err(e) => {
        eprintln!("error! {:#?}", e);
      },
    }
  }
}

fn main() {
  server::spawn();

  let mut client = Client::new_std();

  client.ping("127.0.0.1", 5683)
        .map(|_| println!("pinged ok!\n"))
        .unwrap();

  let req = Req::get("127.0.0.1", 5683, "hello");
  client.send(req).log();

  let mut req = Req::get("127.0.0.1", 5683, "hello");
  req.non();
  client.send(req).log();

  let req = Req::get("127.0.0.1", 5683, "dropped");
  client.send(req).log();

  server::shutdown();
}
