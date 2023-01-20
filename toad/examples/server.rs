use std::io;
use std::sync::{Arc, Mutex};

use toad::config::Config;
use toad::net::Addrd;
use toad::platform::Platform as _;
use toad::req::Req;
use toad::server::ap::state::{Complete, Hydrated};
use toad::server::{path, respond, Ap, BlockingServer, Init};
use toad::std::{dtls, Platform, PlatformTypes as T};
use toad::step::runtime;

fn done(ap: Ap<Hydrated, T<dtls::N>, (), io::Error>) -> Ap<Complete, T<dtls::N>, (), io::Error> {
  #![allow(unreachable_code)]

  ap.pipe(path::check::rest_equals("done"))
    .bind(|_| Ap::respond(panic!("shutting down...")))
}

fn hello(ap: Ap<Hydrated, T<dtls::N>, (), io::Error>) -> Ap<Complete, T<dtls::N>, (), io::Error> {
  ap.pipe(path::segment::check::next_equals("hello"))
    .pipe(path::segment::next(|_, name| {
            name.map(String::from)
                .map(Ap::ok)
                .unwrap_or_else(|| Ap::reject().pretend_unhydrated())
          }))
    .bind(|name| respond::ok(format!("Hello, {name}!").into()))
}

fn not_found(ap: Ap<Hydrated, T<dtls::N>, (), io::Error>)
             -> Ap<Complete, T<dtls::N>, (), io::Error> {
  ap.pipe(path::rest(|_, r| Ap::ok(r.to_string())))
    .bind(|path| respond::not_found(format!("resource {path} not found").into()))
}

type P = Platform<dtls::N, runtime::std::Runtime<dtls::N>>;

pub fn main() {
  std::env::set_var("RUST_LOG", "trace,toad=trace");
  simple_logger::init_with_env().unwrap();

  let (server_addr, client_addr) = ("127.0.0.1:1111", "127.0.0.1:2222");

  let server_starting = Arc::new(Mutex::new(true));
  let server_starting_2 = Arc::clone(&server_starting);

  log::info!("[1] starting server");
  std::thread::spawn(move || {
    let server = P::try_new(server_addr, Config::default()).unwrap();

    let init = Init(Some(|| {
                      *server_starting_2.lock().unwrap() = false;
                    }));

    server.run(init, |run| run.maybe(done).maybe(hello).maybe(not_found))
          .unwrap();
  });

  while *server_starting.lock().unwrap() {
    std::thread::sleep(std::time::Duration::from_millis(10));
  }

  let client = P::try_new(client_addr, Config::default()).unwrap();
  let (_, token) = client.send_msg(Addrd(Req::<T<dtls::N>>::get(server_addr.parse().unwrap(),
                                                                "hello/ethan").into(),
                                         server_addr.parse().unwrap()))
                         .unwrap();
  log::info!("[2] GET /hello/ethan sent");

  let resp = nb::block!(client.poll_resp(token, server_addr.parse().unwrap())).unwrap();
  assert_eq!(resp.data().payload_string().unwrap(),
             "Hello, ethan!".to_string());
  log::info!("[3] got 'Hello, ethan!'");

  let (_, token) = client.send_msg(Addrd(Req::<T<dtls::N>>::get(server_addr.parse().unwrap(),
                                                                "foobar").into(),
                                         server_addr.parse().unwrap()))
                         .unwrap();
  log::info!("[4] GET /foobar sent");

  let resp = nb::block!(client.poll_resp(token, server_addr.parse().unwrap())).unwrap();
  assert_eq!(resp.data().payload_string().unwrap(),
             "resource foobar not found".to_string());
  log::info!("[5] got 'resource foobar not found'");

  client.send_msg(Addrd(Req::<T<dtls::N>>::get(client_addr.parse().unwrap(), "done").into(),
                        server_addr.parse().unwrap()))
        .unwrap();
  log::info!("[6] done");
}
