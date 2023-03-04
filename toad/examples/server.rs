use std::convert::TryFrom;
use std::io;
use std::sync::{Arc, Barrier, Mutex};
use std::time::Duration;

use lazycell::AtomicLazyCell;
use openssl::conf::Conf;
use toad::config::Config;
use toad::net::Addrd;
use toad::platform::Platform as _;
use toad::req::Req;
use toad::server::ap::state::{Complete, Hydrated};
use toad::server::{method, path, respond, Ap, BlockingServer, Init};
use toad::std::{dtls, Platform, PlatformTypes as T};
use toad::step::{runtime, Step};

fn start_server(addr: &'static str) {
  // 5 worker threads + main thread
  static STARTED: AtomicLazyCell<Barrier> = AtomicLazyCell::NONE;
  STARTED.fill(Barrier::new(6)).unwrap();

  log::info!("[1] starting server");
  std::thread::spawn(move || {
    static SERVER: AtomicLazyCell<P> = AtomicLazyCell::NONE;
    SERVER.fill(P::try_new(addr, Config::default()).unwrap())
          .unwrap();

    std::thread::spawn(|| loop {
      SERVER.borrow().unwrap().steps().notify("time").unwrap();
      std::thread::sleep(Duration::from_millis(500));
    });

    for _ in 1..=5 {
      std::thread::spawn(|| {
        let init = Init(Some(|| {
                          STARTED.borrow().unwrap().wait();
                        }));

        SERVER.borrow()
              .unwrap()
              .run(init, |run| {
                run.maybe(route::done)
                   .maybe(route::hello)
                   .maybe(route::time)
                   .maybe(route::not_found)
              })
              .unwrap();
      });
    }
  });

  STARTED.borrow().unwrap().wait();
}

mod route {
  use std::time::{SystemTime, UNIX_EPOCH};

  use toad::time::Millis;
  use toad_msg::MessageOptions;

  use super::*;

  pub fn done(ap: Ap<Hydrated, T<dtls::N>, (), io::Error>)
              -> Ap<Complete, T<dtls::N>, (), io::Error> {
    #![allow(unreachable_code)]

    ap.pipe(method::post)
      .pipe(path::check::rest_equals("done"))
      .bind(|_| Ap::respond(panic!("shutting down...")))
  }

  pub fn hello(ap: Ap<Hydrated, T<dtls::N>, (), io::Error>)
               -> Ap<Complete, T<dtls::N>, (), io::Error> {
    ap.pipe(method::get)
      .pipe(path::segment::check::next_equals("hello"))
      .pipe(path::segment::next(|_, name| {
              name.map(String::from)
                  .map(Ap::ok)
                  .unwrap_or_else(|| Ap::reject().pretend_unhydrated())
            }))
      .bind(|name| respond::ok(format!("Hello, {name}!").into()))
  }

  pub fn time(ap: Ap<Hydrated, T<dtls::N>, (), io::Error>)
              -> Ap<Complete, T<dtls::N>, (), io::Error> {
    ap.pipe(method::get)
      .pipe(path::segment::check::next_equals("time"))
      .bind_hydrated(|_, req| {
        let query = req.data().msg().query::<Vec<_>>().unwrap();
        let (duration_unit, duration_unit_fn) =
          query.into_iter()
               .find(|q| q.starts_with("unit="))
               .map(|s| {
                 s.split('=')
                  .collect::<Vec<_>>()
                  .get(1)
                  .map(|s| s.to_string())
                  .unwrap_or_default()
               })
               .and_then(|unit| match unit.as_str() {
                 | "millis" => Some((unit,
                                     Box::new(|dur: Duration| dur.as_millis())
                                     as Box<dyn Fn(Duration) -> u128>)),
                 | "seconds" => Some((unit, Box::new(|dur: Duration| dur.as_secs() as u128))),
                 | _ => None,
               })
               .unwrap_or_else(|| {
                 ("millis".to_string(), Box::new(|dur: Duration| dur.as_millis()))
               });
        respond::ok(format!(r#"{{ "you_are": {}, "unix_time": {}, "units": "{}" }}"#,
                            req.addr(),
                            duration_unit_fn(SystemTime::now().duration_since(UNIX_EPOCH)
                                                              .unwrap()),
                            duration_unit).into())
      })
  }

  pub fn not_found(ap: Ap<Hydrated, T<dtls::N>, (), io::Error>)
                   -> Ap<Complete, T<dtls::N>, (), io::Error> {
    ap.pipe(path::rest(|_, r| Ap::ok(r.to_string())))
      .bind(|path| respond::not_found(format!("resource {path} not found").into()))
  }
}

mod test {
  use toad_msg::MessageOptions;

  use super::*;

  pub fn not_found(client: &P, addr: &str) {
    let (_, token) = client.send_msg(Addrd(Req::<T<dtls::N>>::get("foobar").into(),
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
    let (_, token) = client.send_msg(Addrd(Req::<T<dtls::N>>::get("hello/ethan").into(),
                                           addr.parse().unwrap()))
                           .unwrap();
    log::info!("[2] GET /hello/ethan sent");

    let resp = nb::block!(client.poll_resp(token, addr.parse().unwrap())).unwrap();
    assert_eq!(resp.data().payload_string().unwrap(),
               "Hello, ethan!".to_string());
    log::info!("[3] got 'Hello, ethan!'");
  }

  pub fn observe(client: &P, addr: &str, unit: &str) {
    let mut register = Req::<T<dtls::N>>::get("time");
    register.msg_mut()
            .set_observe(toad_msg::opt::observe::Action::Register)
            .unwrap();
    register.msg_mut()
            .add_query(format!("unit={unit}"))
            .unwrap();

    let mut deregister = Req::<T<dtls::N>>::get("time");
    deregister.msg_mut()
              .set_observe(toad_msg::opt::observe::Action::Deregister)
              .unwrap();

    let (_, token) = client.send_msg(Addrd(register.into(), addr.parse().unwrap()))
                           .unwrap();
    log::info!("[6] sent GET Observe=1 /time");

    for n in 0..=2 {
      let resp = nb::block!(client.poll_resp(token, addr.parse().unwrap())).unwrap();
      log::info!("[{}] got {}", 7 + n, resp.data().payload_string().unwrap());
    }

    client.send_msg(Addrd(deregister.into(), addr.parse().unwrap()))
          .unwrap();
    log::info!("[10] sent GET Observe=0 /time");
  }
}

type P = Platform<dtls::N, runtime::std::Runtime<dtls::N>>;

pub fn main() {
  std::env::set_var("RUST_LOG", "trace,toad=trace");
  simple_logger::init_with_env().unwrap();

  let (server_addr, client_a_addr, client_b_addr, client_c_addr) =
    ("127.0.0.1:1111", "127.0.0.1:2222", "127.0.0.1:3333", "127.0.0.1:4444");
  start_server(&server_addr);

  let client_a = P::try_new(client_a_addr, Config::default()).unwrap();
  let client_b = P::try_new(client_b_addr, Config::default()).unwrap();
  let client_c = P::try_new(client_c_addr, Config::default()).unwrap();

  let (client_a, client_b, client_c): (&'static P, &'static P, &'static P) = unsafe {
    (core::mem::transmute(&client_a),
     core::mem::transmute(&client_b),
     core::mem::transmute(&client_c))
  };

  test::hello(client_a, server_addr);
  test::not_found(client_a, server_addr);

  let a = std::thread::spawn(|| {
    test::observe(client_a, server_addr, "millis");
  });

  let b = std::thread::spawn(|| {
    test::observe(client_b, server_addr, "millis");
  });

  let c = std::thread::spawn(|| {
    test::observe(client_c, server_addr, "seconds");
  });

  a.join().unwrap();
  b.join().unwrap();
  c.join().unwrap();

  client_a.send_msg(Addrd(Req::<T<dtls::N>>::post("done").into(),
                          server_addr.parse().unwrap()))
          .unwrap();
  log::info!("[10] done");
}
