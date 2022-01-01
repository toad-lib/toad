use std::{fmt::Debug,
          net::UdpSocket,
          sync::{Arc, Barrier},
          thread::{self, JoinHandle}};

use kwap_msg::{EnumerateOptNumbers, TryFromBytes, TryIntoBytes, VecMessage as Message};

fn main() {
  let server_up = Arc::new(Barrier::new(2));
  let _server = spawn_server(server_up.clone());
  server_up.wait();

  let sock = UdpSocket::bind("0.0.0.0:55556").unwrap();
  sock.connect("0.0.0.0:5683").unwrap();
  println!("client: 🔌 connected to server");

  let bytes = loop {
    if let Ok(bytes) = sock.send(&get_hello().try_into_bytes::<Vec<_>>().unwrap()) {
      break bytes;
    }
  };
  println!("client: 📨 sent GET /hello {} bytes", bytes);
  println!("client: 📭 waiting for response...");

  let mut buf = [0; 128];
  let n = sock.recv(&mut buf).unwrap();

  let rep = Message::try_from_bytes(&buf[0..n]).unwrap();
  println!("client: 📨 received {} {}",
           rep.code.to_string(),
           String::from_utf8(rep.payload.0.clone()).unwrap());

  loop {}
}

fn spawn_server(b: Arc<Barrier>) -> JoinHandle<()> {
  thread::spawn(move || {
    let result = || -> Result<(), Box<dyn Debug>> {
      fn err<T: Debug + 'static>(t: T) -> Box<dyn Debug> {
        Box::<_>::from(t)
      }
      let sock = UdpSocket::bind("0.0.0.0:5683").map_err(err)?;
      println!("server: 👂 listening at 0.0.0.0:5683/hello");

      b.wait();

      let mut buf = [0; 128];
      loop {
        let (n, addr) = sock.recv_from(&mut buf).map_err(err)?;
        if n == 0 {
          continue;
        }

        let bytes = &buf[0..n];

        let req = Message::try_from_bytes(bytes).map_err(err)?;

        let method = match req.code.detail {
          | 1 => "GET",
          | 2 => "POST",
          | 3 => "PUT",
          | 4 => "DELETE",
          | _ => unreachable!(),
        };
        let (_, path_opt) = req.opts
                               .iter()
                               .enumerate_option_numbers()
                               .find(|(n, _)| n.0 == 11)
                               .ok_or_else(|| err("no Uri-Path"))?;
        let path = String::from_utf8(path_opt.value.0.clone()).map_err(err)?;

        let rep = match path.as_str() {
          | "hello" => ok_hello(req.token),
          | _ => not_found(req.token),
        };

        println!("server: 📨 got {} {}, sending {}", method, path, rep.code.to_string());

        sock.send_to(&rep.try_into_bytes::<Vec<_>>().unwrap(), addr)
            .map_err(err)?;
      }
    }();

    if let Err(e) = result {
      eprintln!("server: 😞 error {:?}", e);
    }
  })
}

fn get_hello() -> Message {
  use kwap_msg::*;
  Message { id: Id(1),
            ty: Type::Con,
            __optc: Default::default(),
            ver: Default::default(),
            token: Token(Default::default()),
            code: Code { class: 0, detail: 1 }, // GET
            opts: vec![Opt { delta: OptDelta(11), // Uri-Path
                             value: OptValue("hello".as_bytes().to_vec()) }],
            payload: Payload(Vec::new()) }
}

fn ok_hello(token: kwap_msg::Token) -> Message {
  use kwap_msg::*;
  Message { id: Id(1),
            ty: Type::Ack, // ACK
            ver: Default::default(),
            __optc: Default::default(),
            token,
            code: Code { class: 2, detail: 5 }, // 2.05 OK
            opts: Vec::new(),
            payload: Payload("hi there!".as_bytes().to_vec()) }
}

fn not_found(token: kwap_msg::Token) -> Message {
  use kwap_msg::*;
  Message { id: Id(1),
            ty: Type::Ack, // ACK
            ver: Default::default(),
            __optc: Default::default(),
            token,
            code: Code { class: 4, detail: 4 }, // 4.04 NOT FOUND
            opts: Vec::new(),
            payload: Payload("not found :(".as_bytes().to_vec()) }
}
