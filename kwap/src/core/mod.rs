use core::str::FromStr;

use embedded_time::Clock;
use kwap_common::prelude::*;
use kwap_msg::{TryIntoBytes, Type};
use no_std_net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use tinyvec::ArrayVec;

mod error;
/// Core methods that manage inbound messages.
///
/// For core methods that manage outbound messages, see [`outbound`].
mod inbound;
/// Core methods that manage outbound messages.
///
/// For core methods that manage inbound messages, see [`inbound`].
mod outbound;
#[doc(inline)]
pub use error::*;
#[doc(inline)]
pub use inbound::*;
#[doc(inline)]
pub use outbound::*;

use crate::config::{self, Config, Retryable};
use crate::req::Req;
use crate::resp::Resp;
use crate::retry::RetryTimer;
use crate::socket::{Addressed, Socket};

// TODO: support ACK_TIMEOUT, ACK_RANDOM_FACTOR, MAX_RETRANSMIT, NSTART, DEFAULT_LEISURE, PROBING

// Option for these collections provides a Default implementation,
// which is required by ArrayVec.
//
// This also allows us efficiently take owned responses from the collection without reindexing the other elements.
type Buffer<T, const N: usize> = ArrayVec<[Option<T>; N]>;

/// A CoAP request/response runtime that drives client- and server-side behavior.
///
/// Defined as a state machine with state transitions ([`Event`]s).
///
/// The behavior at runtime is fully customizable, with the default behavior provided via [`Core::new()`](#method.new).
#[allow(missing_debug_implementations)]
pub struct Core<Cfg: Config> {
  /// Networking socket that the CoAP runtime uses
  sock: Cfg::Socket,
  /// Clock used for timing
  clock: Cfg::Clock,
  /// Received responses
  resps: Buffer<Addressed<Resp<Cfg>>, 16>,
  /// Queue of messages to send whose receipt we do not need to guarantee (NON, ACK)
  fling_q: Buffer<Addressed<config::Message<Cfg>>, 16>,
  /// Queue of confirmable messages to send at our earliest convenience
  retry_q: Buffer<Retryable<Cfg, Addressed<config::Message<Cfg>>>, 16>,
}

// NOTE!
// This impl is not all the methods available on core.
//
// To reduce code footprint, methods dealing sending messages out are in `outbound`.
//
// Methods that process incoming messages are in `inbound`.
//
// This is probably a smell that Core is too large...
impl<Cfg: Config> Core<Cfg> {
  /// Creates a new Core with the default runtime behavior
  pub fn new(clock: Cfg::Clock, sock: Cfg::Socket) -> Self {
    Self { sock,
           clock,
           resps: Default::default(),
           fling_q: Default::default(),
           retry_q: Default::default() }
  }

  // TODO: use + implement crate-wide logging
  #[allow(dead_code)]
  #[cfg(feature = "std")]
  fn trace_con_q(&self) {
    use kwap_msg::EnumerateOptNumbers;
    self.retry_q
        .iter()
        .filter_map(|o| o.as_ref())
        .for_each(|Retryable(Addressed(con, con_addr), _)| {
          println!("still qd: {con_non:?} {meth} {addr} {route}",
                   con_non = con.ty,
                   meth = con.code.to_string(),
                   addr = con_addr,
                   route = String::from_utf8_lossy(&con.opts
                                                       .iter()
                                                       .enumerate_option_numbers()
                                                       .find(|(num, _)| num.0 == 11)
                                                       .unwrap()
                                                       .1
                                                       .value
                                                       .0
                                                       .iter()
                                                       .copied()
                                                       .collect::<Vec<_>>()));
        });
  }

  #[allow(dead_code)]
  #[cfg(feature = "std")]
  fn trace_non_q(&self) {
    use kwap_msg::EnumerateOptNumbers;
    self.fling_q
        .iter()
        .filter_map(|o| o.as_ref())
        .for_each(|Addressed(con, con_addr)| {
          println!("still qd: {con_non:?} {meth} {addr} {route}",
                   con_non = con.ty,
                   meth = con.code.to_string(),
                   addr = con_addr,
                   route = String::from_utf8_lossy(&con.opts
                                                       .iter()
                                                       .enumerate_option_numbers()
                                                       .find(|(num, _)| num.0 == 11)
                                                       .unwrap()
                                                       .1
                                                       .value
                                                       .0
                                                       .iter()
                                                       .copied()
                                                       .collect::<Vec<_>>()));
        });
  }

  fn retryable<T>(&self, when: When, t: T) -> Result<Retryable<Cfg, T>, Error<Cfg>> {
    self.clock
        .try_now()
        .map(|now| {
          RetryTimer::new(now,
                          crate::retry::Strategy::Exponential(embedded_time::duration::Milliseconds(100)),
                          crate::retry::Attempts(5))
        })
        .map_err(|_| when.what(What::ClockError))
        .map(|timer| Retryable(t, timer))
  }
}

#[cfg(test)]
mod tests {
  use kwap_msg::TryIntoBytes;
  use tinyvec::ArrayVec;

  use super::*;
  use crate::config;
  use crate::config::Alloc;
  use crate::req::Req;
  use crate::test::TubeSock;

  type Config = Alloc<crate::std::Clock, TubeSock>;

  #[test]
  fn ping() {
    type Msg = config::Message<Config>;

    let mut client = Core::<Config>::new(crate::std::Clock::new(), TubeSock::new());
    let (id, addr) = client.ping("0.0.0.0", 5632).unwrap();

    let resp = Msg { id,
                     token: kwap_msg::Token(Default::default()),
                     code: kwap_msg::Code::new(0, 0),
                     ver: Default::default(),
                     ty: kwap_msg::Type::Reset,
                     payload: kwap_msg::Payload(Default::default()),
                     opts: Default::default() };

    let _bytes = resp.try_into_bytes::<ArrayVec<[u8; 1152]>>().unwrap();

    // client.fire(Event::RecvDgram(Some((bytes, addr)))).unwrap();
    client.poll_ping(id, addr).unwrap();
  }

  #[test]
  fn client_flow() {
    type Msg = config::Message<Config>;

    let req = Req::<Config>::get("0.0.0.0", 1234, "");
    let token = req.msg.token;
    let resp = Resp::<Config>::for_request(req);
    let bytes = Msg::from(resp).try_into_bytes::<Vec<u8>>().unwrap();

    let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 1234);
    let mut client = Core::<Config>::new(crate::std::Clock::new(), TubeSock::init(addr.into(), bytes.clone()));

    let rep = client.poll_resp(token, addr.into()).unwrap();
    assert_eq!(bytes, Msg::from(rep).try_into_bytes::<Vec<u8>>().unwrap());
  }
}
