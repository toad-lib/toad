use std::io;
use std::sync::Barrier;

use lazycell::AtomicLazyCell;
use toad::config::Config;
use toad::net::Addrd;
use toad::platform::Platform as _;
use toad::req::Req;
use toad::server::ap::state::{Complete, Hydrated};
use toad::server::{path, respond, Ap, BlockingServer, Init};
use toad::std::{dtls, Platform, PlatformTypes as T};
use toad::step::runtime;

fn start_server(addr: &'static str) {
  const WORKER_THREAD_COUNT: usize = 10;
  static STARTED: AtomicLazyCell<Barrier> = AtomicLazyCell::NONE;
  STARTED.fill(Barrier::new(WORKER_THREAD_COUNT + 1)).unwrap();

  log::info!("[1] starting server");
  std::thread::spawn(move || {
    static SERVER: AtomicLazyCell<P> = AtomicLazyCell::NONE;
    SERVER.fill(P::try_new(addr, Config::default()).unwrap())
          .unwrap();

    for _ in 1..=WORKER_THREAD_COUNT {
      std::thread::spawn(|| {
        let init = Init(Some(|| {
                          STARTED.borrow().unwrap().wait();
                        }));

        SERVER.borrow()
              .unwrap()
              .run(init, |run| {
                run.maybe(route::done)
                   .maybe(route::hello)
                   .maybe(route::not_found)
              })
              .unwrap();
      });
    }
  });

  STARTED.borrow().unwrap().wait();
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

  pub fn hello(client: &P, name: &str, addr: &str) {
    let (_, token) =
      client.send_msg(Addrd(Req::<T<dtls::N>>::get(format!("hello/{}", name)).into(),
                            addr.parse().unwrap()))
            .unwrap();
    log::info!("{} -> GET /hello/{}",
               client.socket().local_addr().unwrap(),
               name);

    let resp = nb::block!(client.poll_resp(token, addr.parse().unwrap())).unwrap();
    assert_eq!(resp.data().payload_string().unwrap(),
               format!("Hello, {}!", name));
    log::info!("<- 'Hello, {}!'", name);
  }
}

type P = Platform<dtls::N, runtime::std::Runtime<dtls::N>>;

pub fn main() {
  std::env::set_var("RUST_LOG", "trace,toad=trace");
  simple_logger::init_with_env().unwrap();

  let server_addr = "127.0.0.1:1111";
  start_server(&server_addr);

  const N_CLIENTS: usize = 4;
  static FINISHED: AtomicLazyCell<Barrier> = AtomicLazyCell::NONE;
  FINISHED.fill(Barrier::new(N_CLIENTS + 1)).unwrap();

  let names = include_str!("./names.txt").split("\n")
                                         .filter(|s| !s.is_empty())
                                         .collect::<Vec<_>>();
  let n_names = names.len();
  let mut names = names.into_iter();
  let names_mut = &mut names;

  for n in 1..=N_CLIENTS {
    let names_count = n_names / N_CLIENTS;
    let names = names_mut.take(names_count).collect::<Vec<_>>();

    std::thread::spawn(move || {
      let addr = format!("127.0.0.1:22{n:02}");
      let client = P::try_new(addr, Config::default()).unwrap();
      names.into_iter()
           .for_each(|name| test::hello(&client, name.trim(), server_addr));
      FINISHED.borrow().unwrap().wait();
    });
  }

  FINISHED.borrow().unwrap().wait();

  let done = Addrd(Req::<T<dtls::N>>::get("done").into(),
                   server_addr.parse().unwrap());

  P::try_new("127.0.0.1:8888", Config::default()).unwrap()
                                                 .send_msg(done)
                                                 .unwrap();
}
