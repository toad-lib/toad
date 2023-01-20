use toad::config::Config;
use toad::net::Addrd;
use toad::platform::Platform as _;
use toad::req::Req;
use toad::std::{dtls, Platform, PlatformTypes as T};
use toad::step::runtime;

pub fn main() {
  simple_logger::init().unwrap();

  let (server_addr, client_addr) = ("0.0.0.0:1111", "0.0.0.0:2222");

  type P = Platform<dtls::N, runtime::std::Runtime<dtls::N>>;

  let server = P::try_new(server_addr, Config::default()).unwrap();

  let client = P::try_new(client_addr, Config::default()).unwrap();
  client.send_msg(Addrd(Req::<T<dtls::N>>::get(server_addr.parse().unwrap(), "hello").into(),
                        server_addr.parse().unwrap()))
        .unwrap();

  let req = nb::block!(server.poll_req()).unwrap();
  assert_eq!(req.data().path().ok().flatten(), Some("hello"));
}
