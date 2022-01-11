use std::net::UdpSocket;

use kwap::config::Alloc;
use kwap::core::Core;
use kwap::req::Req;

fn main() {
  let sock = UdpSocket::bind("0.0.0.0:4870").unwrap();
  println!("bound to 0.0.0.0:4870\n");
  let mut core = Core::<UdpSocket, Alloc>::new(sock);
  let req = Req::<Alloc>::get("0.0.0.0", 5683, "hello");

  get_hello(&mut core, req.clone());
  println!();
  get_hello(&mut core, req);

  fn get_hello(core: &mut Core<UdpSocket, Alloc>, req: Req<Alloc>) {
    let id = req.msg_id();
    core.send_req(req).unwrap();
    println!("GET 0.0.0.0:5683/hello");

    let resp = nb::block!(core.poll_resp(id));

    match resp {
      | Ok(rep) => {
        println!("{} {:?}", rep.code().to_string(), rep.payload_string().unwrap());
      },
      | Err(e) => {
        eprintln!("error! {:#?}", e);
      },
    }
  }
}
