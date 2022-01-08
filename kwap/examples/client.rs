use kwap::req::Req;
use kwap::core::Core;
use kwap::config::Alloc;
use std::net::UdpSocket;

fn main() {
  let sock = UdpSocket::bind("0.0.0.0:4870").unwrap();
  println!("bound to 0.0.0.0:4870\n");
  let mut core = Core::<UdpSocket, Alloc>::new(sock);
  let req = Req::<Alloc>::get("0.0.0.0", 5683, "hello");

  get_hello(&mut core, req.clone());
  println!();
  get_hello(&mut core, req);

  fn get_hello(core: &mut Core<UdpSocket, Alloc>, req: Req::<Alloc>) {
  let id = req.msg_id();
  core.send_req(req).unwrap();
  println!("GET 0.0.0.0:5683/hello");

  loop {
    match core.poll_resp(id) {
      Ok(rep) => {
        println!("{} {:?}", rep.code().to_string(), rep.payload_string().unwrap());
        break;
      },
      Err(nb::Error::WouldBlock) => {
        println!("waiting...");
        std::thread::sleep(std::time::Duration::from_millis(500));
      },
      Err(e) => {
        eprintln!("error! {:#?}", e);
        break;
      },
    }
  }
  }

}
