#![allow(dead_code)]

use ::core::cell::Cell;
use ::core::ops::Deref;
use ::core::time::Duration;
use ::std::sync::{Mutex, RwLock};
use ::std::thread;
use ::toad_msg::{TryFromBytes, TryIntoBytes};
use embedded_time::rate::Fraction;
use embedded_time::Instant;
use net::*;
use no_std_net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std_alloc::sync::Arc;
use tinyvec::ArrayVec;
use toad_msg::Token;
use toad_stem::Stem;

use super::*;

// lol `crate::test::x.x.x.x(80)`
pub struct X1 {
  pub x: X2,
}
pub struct X2 {
  pub x: X3,
}
pub struct X3;
impl X3 {
  pub fn x(&self, port: u16) -> SocketAddr {
    addr(port)
  }
}

#[allow(non_upper_case_globals)]
pub const x: X1 = X1 { x: X2 { x: X3 } };

pub fn addr(port: u16) -> SocketAddr {
  use no_std_net::*;
  SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(192, 168, 0, 1), port))
}

#[macro_export]
macro_rules! msg {
  (CON GET x.x.x.x:$port:literal) => { $crate::test::msg!(CON {0 . 1} x.x.x.x:$port) };
  (CON PUT x.x.x.x:$port:literal) => { $crate::test::msg!(CON {0 . 2} x.x.x.x:$port) };
  (CON POST x.x.x.x:$port:literal) => { $crate::test::msg!(CON {0 . 3} x.x.x.x:$port) };
  (CON DELETE x.x.x.x:$port:literal) => { $crate::test::msg!(CON {0 . 4} x.x.x.x:$port) };
  (NON GET x.x.x.x:$port:literal) => { $crate::test::msg!(NON {0 . 1} x.x.x.x:$port) };
  (NON PUT x.x.x.x:$port:literal) => { $crate::test::msg!(NON {0 . 2} x.x.x.x:$port) };
  (NON POST x.x.x.x:$port:literal) => { $crate::test::msg!(NON {0 . 3} x.x.x.x:$port) };
  (NON DELETE x.x.x.x:$port:literal) => { $crate::test::msg!(NON {0 . 4} x.x.x.x:$port) };

  (CON {$c:literal . $d:literal} x.x.x.x:$port:literal $(with $f:expr)?) => {{
    $crate::test::msg!({::toad_msg::Type::Con} {::toad_msg::Code::new($c, $d)} x.x.x.x:$port $(with $f)?)
  }};
  (NON {$c:literal . $d:literal} x.x.x.x:$port:literal $(with $f:expr)?) => {{
    $crate::test::msg!({::toad_msg::Type::Non} {::toad_msg::Code::new($c, $d)} x.x.x.x:$port $(with $f)?)
  }};
  (ACK {$c:literal . $d:literal} x.x.x.x:$port:literal $(with $f:expr)?) => {{
    $crate::test::msg!({::toad_msg::Type::Ack} {::toad_msg::Code::new($c, $d)} x.x.x.x:$port $(with $f)?)
  }};
  (ACK EMPTY x.x.x.x:$port:literal) => {{
    $crate::test::msg!({::toad_msg::Type::Ack} {::toad_msg::Code::new(0, 0)} x.x.x.x:$port)
  }};

  (RESET x.x.x.x:$port:literal) => {{
    $crate::test::msg!({::toad_msg::Type::Reset} {::toad_msg::Code::new(0, 0)} x.x.x.x:$port)
  }};

  ({$ty:expr} {$code:expr} x.x.x.x:$port:literal $(with $f:expr)?) => {{
    use $crate::net::Addrd;
    use ::toad_msg::*;

    let addr = $crate::test::x.x.x.x($port);

    #[allow(unused_mut)]
    let mut msg = Addrd($crate::test::Message {
      ver: Default::default(),
      ty: $ty,
      token: Token(Default::default()),
      code: $code,
      id: Id(0),
      opts: Default::default(),
      payload: Payload(Default::default()),
    }, addr);

    $($f(msg.as_mut());)?

    msg
  }};
}

pub use msg;

pub type Message = crate::platform::Message<Platform>;
pub type Snapshot = crate::platform::Snapshot<Platform>;
pub type Effect = crate::platform::Effect<Platform>;
pub type Req = crate::req::Req<Platform>;
pub type Resp = crate::resp::Resp<Platform>;

pub fn snapshot() -> Snapshot {
  Snapshot { config: Default::default(),
             time: ClockMock::instant(0),
             recvd_dgram: None }
}

pub fn dummy_addr() -> SocketAddr {
  SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(192, 168, 0, 1), 8080))
}

pub fn dummy_addr_2() -> SocketAddr {
  SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(192, 168, 0, 2), 8080))
}

pub fn dummy_addr_3() -> SocketAddr {
  SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(192, 168, 0, 3), 8080))
}

pub mod stepfn {
  #![allow(non_camel_case_types)]
  use super::*;

  pub trait poll_req<Self_, Req, E>
    where Self: 'static
            + for<'a> FnMut(&'a Self_,
                          &'a Snapshot,
                          &'a mut Vec<Effect>)
                          -> Option<nb::Result<Req, E>>
  {
  }
  impl<T, Self_, Req, E> poll_req<Self_, Req, E> for T
    where T: 'static
            + for<'a> FnMut(&'a Self_,
                          &'a Snapshot,
                          &'a mut Vec<Effect>)
                          -> Option<nb::Result<Req, E>>
  {
  }

  pub trait poll_resp<Self_, Resp, E>
    where Self: 'static
            + for<'a> FnMut(&'a Self_,
                          &'a Snapshot,
                          &'a mut Vec<Effect>,
                          Token,
                          SocketAddr) -> Option<nb::Result<Resp, E>>
  {
  }
  impl<T, Self_, Resp, E> poll_resp<Self_, Resp, E> for T
    where T: 'static
            + for<'a> FnMut(&'a Self_,
                          &'a Snapshot,
                          &'a mut Vec<Effect>,
                          Token,
                          SocketAddr) -> Option<nb::Result<Resp, E>>
  {
  }

  pub trait notify<Self_, E>
    where Self: 'static + for<'a> FnMut(&'a Self_, &'a str, &'a mut Vec<Effect>) -> Result<(), E>
  {
  }
  impl<T, Self_, E> notify<Self_, E> for T
    where T: 'static + for<'a> FnMut(&'a Self_, &'a str, &'a mut Vec<Effect>) -> Result<(), E>
  {
  }

  pub trait before_message_sent<Self_, E>
    where Self: 'static
            + for<'a> FnMut(&'a Self_,
                          &'a Snapshot,
                          &'a mut Vec<Effect>,
                          &'a mut Addrd<Message>) -> Result<(), E>
  {
  }
  impl<T, Self_, E> before_message_sent<Self_, E> for T
    where T: 'static
            + for<'a> FnMut(&'a Self_,
                          &'a Snapshot,
                          &'a mut Vec<Effect>,
                          &'a mut Addrd<Message>) -> Result<(), E>
  {
  }
  pub trait on_message_sent<Self_, E>
    where Self: 'static
            + for<'a> FnMut(&'a Self_,
                          &'a Snapshot,
                          &'a mut Vec<Effect>,
                          &'a Addrd<Message>) -> Result<(), E>
  {
  }
  impl<T, Self_, E> on_message_sent<Self_, E> for T
    where T: 'static
            + for<'a> FnMut(&'a Self_,
                          &'a Snapshot,
                          &'a mut Vec<Effect>,
                          &'a Addrd<Message>) -> Result<(), E>
  {
  }
}

pub struct MockStep<State, Req, Resp, E> {
  pub poll_req: RwLock<Box<dyn stepfn::poll_req<Self, Req, E>>>,
  pub poll_resp: RwLock<Box<dyn stepfn::poll_resp<Self, Resp, E>>>,
  pub notify: RwLock<Box<dyn stepfn::notify<Self, E>>>,
  pub before_message_sent: RwLock<Box<dyn stepfn::before_message_sent<Self, E>>>,
  pub on_message_sent: RwLock<Box<dyn stepfn::on_message_sent<Self, E>>>,
  pub state: Stem<Option<State>>,
}

impl<State, Rq, Rp, E> MockStep<State, Rq, Rp, E> {
  pub fn init(&self, new: State) -> &Self {
    let mut new = Some(new);
    self.state.map_mut(|o| *o = new.take());
    self
  }

  pub fn init_default(&self) -> &Self
    where State: Default
  {
    self.init(Default::default())
  }

  pub fn set_poll_req(&self, f: impl stepfn::poll_req<Self, Rq, E>) -> &Self {
    let mut g = self.poll_req.try_write().unwrap();
    *g = Box::new(f);
    self
  }

  pub fn set_poll_resp(&self, f: impl stepfn::poll_resp<Self, Rp, E>) -> &Self {
    let mut g = self.poll_resp.try_write().unwrap();
    *g = Box::new(f);
    self
  }

  pub fn set_notify(&self, f: impl stepfn::notify<Self, E>) -> &Self {
    let mut g = self.notify.try_write().unwrap();
    *g = Box::new(f);
    self
  }

  pub fn set_before_message_sent(&self, f: impl stepfn::before_message_sent<Self, E>) -> &Self {
    let mut g = self.before_message_sent.try_write().unwrap();
    *g = Box::new(f);
    self
  }

  pub fn set_on_message_sent(&self, f: impl stepfn::on_message_sent<Self, E>) -> &Self {
    let mut g = self.on_message_sent.try_write().unwrap();
    *g = Box::new(f);
    self
  }
}

impl<State, Rq, Rp, E> Default for MockStep<State, Rq, Rp, E> {
  fn default() -> Self {
    Self { poll_req: RwLock::new(Box::new(|_, _, _| None)),
           poll_resp: RwLock::new(Box::new(|_, _, _, _, _| None)),
           notify: RwLock::new(Box::new(|_, _, _| Ok(()))),
           before_message_sent: RwLock::new(Box::new(|_, _, _, _| Ok(()))),
           on_message_sent: RwLock::new(Box::new(|_, _, _, _| Ok(()))),
           state: Stem::new(None) }
  }
}

impl<State, Rq, Rp, E> crate::step::Step<Platform> for MockStep<State, Rq, Rp, E>
  where E: From<()> + crate::step::Error
{
  type PollReq = Rq;
  type PollResp = Rp;
  type Error = E;
  type Inner = ();

  fn inner(&self) -> &Self::Inner {
    &()
  }

  fn poll_req(&self,
              snap: &platform::Snapshot<Platform>,
              effects: &mut <Platform as platform::PlatformTypes>::Effects)
              -> step::StepOutput<Self::PollReq, Self::Error> {
    let mut g = self.poll_req.try_write().unwrap();
    g.as_mut()(self, snap, effects)
  }

  fn poll_resp(&self,
               snap: &platform::Snapshot<Platform>,
               effects: &mut <Platform as platform::PlatformTypes>::Effects,
               token: Token,
               addr: SocketAddr)
               -> step::StepOutput<Self::PollResp, Self::Error> {
    let mut g = self.poll_resp.try_write().unwrap();
    g.as_mut()(self, snap, effects, token, addr)
  }

  fn notify<Path>(&self, path: Path, effects: &mut Vec<Effect>) -> Result<(), Self::Error>
    where Path: AsRef<str> + Clone
  {
    let mut g = self.notify.try_write().unwrap();
    g.as_mut()(self, path.as_ref(), effects)
  }

  fn before_message_sent(&self,
                         snap: &platform::Snapshot<Platform>,
                         effects: &mut <Platform as platform::PlatformTypes>::Effects,
                         msg: &mut Addrd<platform::Message<Platform>>)
                         -> Result<(), Self::Error> {
    let mut g = self.before_message_sent.try_write().unwrap();
    g.as_mut()(self, snap, effects, msg)
  }

  fn on_message_sent(&self,
                     snap: &platform::Snapshot<Platform>,
                     effects: &mut Vec<Effect>,
                     msg: &Addrd<platform::Message<Platform>>)
                     -> Result<(), Self::Error> {
    let mut g = self.on_message_sent.try_write().unwrap();
    g.as_mut()(self, snap, effects, msg)
  }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TimeoutState {
  Canceled,
  WillPanic,
}

pub struct Timeout {
  pub state: Arc<Mutex<TimeoutState>>,
  dur: Duration,
}

impl Timeout {
  pub fn new(dur: Duration) -> Self {
    Self { state: Arc::new(Mutex::new(TimeoutState::WillPanic)),
           dur }
  }

  pub fn cancel(state: Arc<Mutex<TimeoutState>>) {
    *state.lock().unwrap() = TimeoutState::Canceled;
  }

  pub fn wait(&self) {
    if self.state.lock().unwrap().deref() == &TimeoutState::Canceled {
      return;
    };

    thread::sleep(self.dur);
    if self.state.lock().unwrap().deref() == &TimeoutState::WillPanic {
      panic!("test timed out");
    } else {
      ()
    }
  }
}

/// Config implementor using mocks for clock and sock
pub type Platform = crate::platform::Alloc<ClockMock, SockMock>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ClockMock(pub Cell<u64>);

impl ClockMock {
  pub fn new() -> Self {
    Self(Cell::new(0))
  }

  pub fn set(&self, to: u64) {
    self.0.set(to);
  }

  pub fn instant(n: u64) -> Instant<Self> {
    Instant::new(n)
  }
}

impl embedded_time::Clock for ClockMock {
  type T = u64;

  const SCALING_FACTOR: Fraction = Fraction::new(1, 1_000_000);

  fn try_now(&self) -> Result<Instant<Self>, embedded_time::clock::Error> {
    Ok(Instant::new(self.0.get()))
  }
}

/// A mocked socket
#[derive(Debug)]
pub struct SockMock {
  /// Inbound bytes from remote sockets. Address represents the sender
  pub rx: Arc<Mutex<Vec<Addrd<Vec<u8>>>>>,
  /// Outbound bytes to remote sockets. Address represents the destination
  pub tx: Arc<Mutex<Vec<Addrd<Vec<u8>>>>>,
}

impl SockMock {
  pub fn new() -> Self {
    Self { rx: Default::default(),
           tx: Default::default() }
  }

  pub fn send_msg<P: platform::PlatformTypes>(rx: &Arc<Mutex<Vec<Addrd<Vec<u8>>>>>,
                                              msg: Addrd<platform::Message<P>>) {
    rx.lock()
      .unwrap()
      .push(msg.map(|msg| msg.try_into_bytes().unwrap()));
  }

  pub fn await_msg<P: platform::PlatformTypes>(addr: SocketAddr,
                                               tx: &Arc<Mutex<Vec<Addrd<Vec<u8>>>>>)
                                               -> platform::Message<P> {
    let attempt = || {
      tx.lock()
        .unwrap()
        .iter_mut()
        .find(|bytes| bytes.addr() == addr && !bytes.data().is_empty())
        .map(|Addrd(bytes, _)| {
          platform::Message::<P>::try_from_bytes(bytes.drain(..).collect::<Vec<_>>()).unwrap()
        })
    };

    loop {
      if let Some(msg) = attempt() {
        break msg;
      }
    }
  }
}

impl Socket for SockMock {
  type Error = Option<()>;
  type Dgram = ArrayVec<[u8; 1024]>;

  fn empty_dgram() -> Self::Dgram {
    ArrayVec::from([0u8; 1024])
  }

  fn recv(&self, buf: &mut [u8]) -> nb::Result<Addrd<usize>, Self::Error> {
    let mut rx = self.rx.lock().unwrap();

    if rx.is_empty() {
      return Err(nb::Error::WouldBlock);
    }

    let dgram = rx.drain(0..1).next().unwrap();

    dgram.data()
         .iter()
         .enumerate()
         .for_each(|(ix, byte)| buf[ix] = *byte);

    Ok(dgram.map(|bytes| bytes.len()))
  }

  fn send(&self, buf: Addrd<&[u8]>) -> nb::Result<(), Self::Error> {
    let mut vec = self.tx.lock().unwrap();
    vec.push(buf.map(Vec::from));
    Ok(())
  }

  fn join_multicast(&self, _: no_std_net::IpAddr) -> Result<(), Self::Error> {
    todo!()
  }

  fn bind_raw<A: no_std_net::ToSocketAddrs>(_: A) -> Result<Self, Self::Error> {
    Ok(Self::new())
  }

  fn peek(&self, _: &mut [u8]) -> nb::Result<Addrd<usize>, Self::Error> {
    todo!()
  }

  fn local_addr(&self) -> SocketAddr {
    todo!()
  }
}

#[test]
#[should_panic]
fn times_out() {
  let timeout = Timeout::new(Duration::from_millis(100));
  thread::spawn(|| thread::sleep(Duration::from_millis(110)));
  timeout.wait();
}

#[test]
fn doesnt_time_out() {
  let timeout = Timeout::new(Duration::from_secs(1));
  let state = timeout.state.clone();
  Timeout::cancel(state);
  timeout.wait();
}
