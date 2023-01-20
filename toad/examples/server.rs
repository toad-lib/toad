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

  fn start_server(addr: &'static str) {
  let server_starting = Arc::new(Mutex::new(true));
  let server_starting_2 = Arc::clone(&server_starting);

  log::info!("[1] starting server");
  std::thread::spawn(move || {
    let server = P::try_new(addr, Config::default()).unwrap();

    let init = Init(Some(|| {
                      *server_starting_2.lock().unwrap() = false;
                    }));

    server.run(init, |run| {
            run.maybe(route::done)
               .maybe(route::hello)
               .maybe(route::not_found)
          })
          .unwrap();
  });

  while *server_starting.lock().unwrap() {
    std::thread::sleep(std::time::Duration::from_millis(10));
  }
  }


mod route {
  use super::*;

  pub fn done(ap: Ap<Hydrated, T<dtls::N>, (), io::Error>)
              -> Ap<Complete, T<dtls::N>, (), io::Error> {
    #![allow(unreachable_code)]

    ap.pipe(path::check::rest_equals("done"))
      .bind(|_| Ap::respond(panic!("shutting down...")))
  }

  pub fn hello(ap: Ap<Hydrated, T<dtls::N>, (), io::Error>)
               -> Ap<Complete, T<dtls::N>, (), io::Error> {
    ap.pipe(path::segment::check::next_equals("hello"))
      .pipe(path::segment::next(|_, name| {
              name.map(String::from)
                  .map(Ap::ok)
                  .unwrap_or_else(|| Ap::reject().pretend_unhydrated())
            }))
      .bind(|name| respond::ok(format!("Hello, {name}!").into()))
  }

  pub fn not_found(ap: Ap<Hydrated, T<dtls::N>, (), io::Error>)
                   -> Ap<Complete, T<dtls::N>, (), io::Error> {
    ap.pipe(path::rest(|_, r| Ap::ok(r.to_string())))
      .bind(|path| respond::not_found(format!("resource {path} not found").into()))
  }
}

mod test {
  use super::*;

  pub fn not_found(client: &P, addr: &str) {
    // UX: why do i have to specify the address twice, when i make the request and when i invoke send_msg?
    let (_, token) = client.send_msg(Addrd(Req::<T<dtls::N>>::get(addr.parse().unwrap(),
                                                                  "foobar").into(),
                                           addr.parse().unwrap()))
                           .unwrap();
    log::info!("[4] GET /foobar sent");

    // UX: why do i have to nb::block?
    let resp = nb::block!(client.poll_resp(token, addr.parse().unwrap())).unwrap();
    assert_eq!(resp.data().payload_string().unwrap(),
               "resource foobar not found".to_string());
    log::info!("[5] got 'resource foobar not found'");
  }

pub fn hello(client: &P, addr: &str) {
  let (_, token) = client.send_msg(Addrd(Req::<T<dtls::N>>::get(addr.parse().unwrap(),
                                                                "hello/ethan").into(),
                                         addr.parse().unwrap()))
                         .unwrap();
  log::info!("[2] GET /hello/ethan sent");

  let resp = nb::block!(client.poll_resp(token, addr.parse().unwrap())).unwrap();
  assert_eq!(resp.data().payload_string().unwrap(),
             "Hello, ethan!".to_string());
  log::info!("[3] got 'Hello, ethan!'");
}
}

type P = Platform<dtls::N, runtime::std::Runtime<dtls::N>>;

pub fn main() {
  std::env::set_var("RUST_LOG", "trace,toad=trace");
  simple_logger::init_with_env().unwrap();

  let (server_addr, client_addr) = ("127.0.0.1:1111", "127.0.0.1:2222");
  start_server(&server_addr);

  let client = P::try_new(client_addr, Config::default()).unwrap();
  test::hello(&client, server_addr);
  test::not_found(&client, server_addr);

  client.send_msg(Addrd(Req::<T<dtls::N>>::get(client_addr.parse().unwrap(), "done").into(),
                        server_addr.parse().unwrap()))
        .unwrap();
  log::info!("[6] done");
}
