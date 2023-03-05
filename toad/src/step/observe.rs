use core::fmt::Debug;
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;

use no_std_net::SocketAddr;
use toad_common::hash::Blake2Hasher;
use toad_common::{Array, Stem};
use toad_msg::opt::known::observe::Action::{Deregister, Register};
use toad_msg::opt::known::repeat::QUERY;
use toad_msg::repeat::PATH;
use toad_msg::{CodeKind, Id, MessageOptions, Token};

use super::Step;
use crate::net::Addrd;
use crate::platform::{self, Effect, PlatformTypes};
use crate::req::Req;
use crate::resp::Resp;

/// Custom metadata options used to track messages created by this step.
///
/// These options will always be stripped from outbound messages before sending.
pub mod opt {
  use toad_msg::OptNumber;

  /// The presence of this option indicates that this message was
  /// created by the [`super::Observe`] step and should not, under
  /// any circumstances, trigger any additional message creation.
  pub const WAS_CREATED_BY_OBSERVE: OptNumber = OptNumber(65000);
}

/// Default hasher used for [`SubscriptionHash`]
///
/// Hashes:
///  - [Message Type](toad_msg::Message.ty)
///  - [Uri-Path](toad_msg::opt::known::no_repeat::HOST)
///  - [Uri-Query](toad_msg::opt::known::no_repeat::HOST)
///  - [Accept](toad_msg::opt::known::no_repeat::ACCEPT)
#[derive(Debug, Clone)]
#[allow(non_camel_case_types)]
pub struct SubHash_TypePathQueryAccept<P>(Blake2Hasher, PhantomData<P>);

impl<P> Default for SubHash_TypePathQueryAccept<P> {
  fn default() -> Self {
    Self(Blake2Hasher::new(), PhantomData)
  }
}

impl<P> SubHash_TypePathQueryAccept<P> {
  /// Create a new `DefaultSubscriptionHasher`
  pub fn new() -> Self {
    Self::default()
  }
}

impl<P> SubscriptionHash<P> for SubHash_TypePathQueryAccept<P> where P: PlatformTypes
{
  type Hasher = Blake2Hasher;

  fn hasher(&mut self) -> &mut Self::Hasher {
    &mut self.0
  }

  fn subscription_hash(&mut self, sub: &Addrd<Req<P>>) {
    let msg = sub.data().msg();

    msg.ty.hash(&mut self.0);
    msg.get(QUERY).into_iter().for_each(|v| {
                                v.hash(&mut self.0);
                              });
    msg.accept().hash(&mut self.0);
    msg.get(PATH).into_iter().for_each(|v| {
                               v.hash(&mut self.0);
                             });
  }
}

/// Extends [`core::hash::Hash`] with "subscription similarity"
/// used to determine whether similar subscriptions may be grouped together.
///
/// A default implementation is provided by [`SubHash_TypePathQueryAccept`].
///
/// ## Why?
/// When your server [`notify`](super::Step::notify)s the toad runtime
/// that there is a new version of a resource available, all
/// subscriptions matching the path passed to `notify` will be
/// re-sent as new requests to your server.
///
/// Similar requests (determined by this trait) will be grouped together
/// so that your server only sees 1 request, and the response
/// will be fanned back out to the subscribers.
///
/// For a more concrete example, see the [module documentation](self).
pub trait SubscriptionHash<P>
  where Self: Sized + Debug,
        P: PlatformTypes
{
  /// Type used to generate hashes
  type Hasher: Hasher;

  #[allow(missing_docs)]
  fn hasher(&mut self) -> &mut Self::Hasher;

  /// Mutate the hasher instance with a subscription
  ///
  /// To obtain the [`u64`] hash, use [`Hasher::finish`] on [`sub_hash.hasher()`](SubscriptionHash::hasher)
  ///
  /// ```
  /// use core::hash::Hasher;
  ///
  /// use toad::net::{ipv4_socketaddr, Addrd};
  /// use toad::platform::toad_msg::Message;
  /// use toad::req::Req;
  /// use toad::step::observe::{SubHash_TypePathQueryAccept, SubscriptionHash};
  /// use toad_msg::Type::Con;
  /// use toad_msg::{Code, Id, Token};
  ///
  /// type Std = toad::std::PlatformTypes<toad::std::dtls::N>;
  ///
  /// let msg_a = Message::<Std>::new(Con, Code::GET, Id(1), Token(Default::default()));
  /// let req_a = Addrd(Req::<Std>::from(msg_a),
  ///                   ipv4_socketaddr([127, 0, 0, 1], 1234));
  /// let mut ha = SubHash_TypePathQueryAccept::new();
  /// ha.subscription_hash(&req_a);
  ///
  /// let msg_b = Message::<Std>::new(Con, Code::GET, Id(2), Token(Default::default()));
  /// let req_b = Addrd(Req::<Std>::from(msg_b),
  ///                   ipv4_socketaddr([127, 0, 0, 1], 2345));
  /// let mut hb = SubHash_TypePathQueryAccept::new();
  /// hb.subscription_hash(&req_a);
  ///
  /// assert_eq!(ha.hasher().finish(), hb.hasher().finish());
  /// ```
  fn subscription_hash(&mut self, sub: &Addrd<Req<P>>);
}

impl<P, T> SubscriptionHash<P> for &mut T
  where P: PlatformTypes,
        T: SubscriptionHash<P>
{
  type Hasher = T::Hasher;

  fn hasher(&mut self) -> &mut Self::Hasher {
    <T as SubscriptionHash<P>>::hasher(self)
  }

  fn subscription_hash(&mut self, sub: &Addrd<Req<P>>) {
    <T as SubscriptionHash<P>>::subscription_hash(self, sub)
  }
}

/// An Observe subscription
pub struct Sub<P>
  where P: PlatformTypes
{
  req: Addrd<Req<P>>,
}

impl<P> core::fmt::Debug for Sub<P> where P: PlatformTypes
{
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_struct("Sub").field("req", &self.req).finish()
  }
}

impl<P> Sub<P> where P: PlatformTypes
{
  #[allow(missing_docs)]
  pub fn new(req: Addrd<Req<P>>) -> Self {
    Self { req }
  }

  #[allow(missing_docs)]
  pub fn addr(&self) -> SocketAddr {
    self.req.addr()
  }

  #[allow(missing_docs)]
  pub fn unwrap(self) -> Addrd<Req<P>> {
    self.req
  }

  /// Get a reference to the request this subscription
  /// originated from
  pub fn req(&self) -> &Addrd<Req<P>> {
    &self.req
  }

  #[allow(missing_docs)]
  pub fn msg(&self) -> &platform::Message<P> {
    &self.req.data().msg()
  }

  #[allow(missing_docs)]
  pub fn id(&self) -> Id {
    self.msg().id
  }

  #[allow(missing_docs)]
  pub fn token(&self) -> Token {
    self.msg().token
  }
}

/// See [the module documentation](self)
#[derive(Debug)]
pub struct Observe<S, Subs, RequestQueue, Hasher> {
  inner: S,
  subs: Stem<Subs>,
  request_queue: Stem<RequestQueue>,
  __hasher: PhantomData<Hasher>,
}

impl<I, S, RQ, H> Default for Observe<I, S, RQ, H>
  where I: Default,
        S: Default,
        RQ: Default
{
  fn default() -> Self {
    Observe { inner: I::default(),
              subs: Stem::new(S::default()),
              request_queue: Stem::new(RQ::default()),
              __hasher: PhantomData }
  }
}

impl<S, Subs, RequestQueue, Hasher> Observe<S, Subs, RequestQueue, Hasher> {
  fn hash<'a, P>(sub: &'a Sub<P>) -> (&'a Sub<P>, u64)
    where P: PlatformTypes,
          Hasher: SubscriptionHash<P> + Default
  {
    (sub, Self::hash_req(sub.req()))
  }

  fn hash_req<'a, P>(sub: &'a Addrd<Req<P>>) -> u64
    where P: PlatformTypes,
          Hasher: SubscriptionHash<P> + Default
  {
    let mut h = Hasher::default();
    h.subscription_hash(sub);
    h.hasher().finish()
  }

  fn get<'a, P>(subs: &'a Subs, addr: SocketAddr, t: Token) -> Option<&'a Sub<P>>
    where Subs: Array<Item = Sub<P>>,
          P: PlatformTypes
  {
    subs.iter().find(|s| s.token() == t && s.addr() == addr)
  }

  fn get_index<'a, P>(subs: &'a Subs, t: Token) -> Option<usize>
    where Subs: Array<Item = Sub<P>>,
          P: PlatformTypes
  {
    subs.iter()
        .enumerate()
        .find(|(_, s)| s.token() == t)
        .map(|(ix, _)| ix)
  }

  fn similar_to<'a, P>(subs: &'a Subs,
                       addr: SocketAddr,
                       t: Token)
                       -> impl 'a + Iterator<Item = &'a Sub<P>>
    where Subs: Array<Item = Sub<P>>,
          P: PlatformTypes,
          Hasher: SubscriptionHash<P> + Default
  {
    subs.iter()
        .filter(move |s| match Self::get(subs, addr, t).map(Self::hash) {
          | Some((sub, h)) => {
            s.addr() != sub.addr() && s.token() != sub.token() && Self::hash(sub).1 == h
          },
          | None => false,
        })
  }

  fn subs_matching_path<'a, 'b, P>(subs: &'a Subs,
                                   p: &'b str)
                                   -> impl 'a + Iterator<Item = &'a Sub<P>>
    where Subs: Array<Item = Sub<P>>,
          P: PlatformTypes,
          'b: 'a
  {
    subs.iter().filter(move |s| {
                 s.msg()
                  .get(PATH)
                  .map(|segs| {
                    segs.iter()
                        .map(|val| -> &[u8] { &val.0 })
                        .eq(p.split("/").map(|s| s.as_bytes()))
                  })
                  .unwrap_or_else(|| p.is_empty())
               })
  }

  fn remove_queued_requests_matching_path<P>(rq: &mut RequestQueue, path: &str) -> ()
    where P: PlatformTypes,
          RequestQueue: Array<Item = Addrd<Req<P>>>
  {
    fn go<P, RQ>(rq: &mut RQ, p: &str) -> ()
      where P: PlatformTypes,
            RQ: Array<Item = Addrd<Req<P>>>
    {
      match rq.iter()
              .enumerate()
              .find(|(_, req)| {
                req.data()
                   .msg()
                   .get(PATH)
                   .map(|segs| {
                     segs.iter()
                         .map(|val| -> &[u8] { &val.0 })
                         .eq(p.split("/").map(|s| s.as_bytes()))
                   })
                   .unwrap_or_else(|| p.is_empty())
              })
              .map(|(ix, _)| ix)
      {
        | Some(ix) => {
          rq.remove(ix);
          go(rq, p);
        },
        | None => (),
      }
    }

    go::<P, RequestQueue>(rq, path)
  }

  fn get_queued_request<P>(&self) -> Option<Addrd<Req<P>>>
    where P: PlatformTypes,
          RequestQueue: Array<Item = Addrd<Req<P>>>
  {
    self.request_queue.map_mut(|rq| {
                        if rq.is_empty() {
                          None
                        } else {
                          rq.remove(rq.len() - 1)
                        }
                      })
  }

  // [1a]: Observe=1?
  // [2a]: add to subs
  // [3a]: pass request up to server
  fn handle_incoming_request<P, E>(&self,
                                   req: Addrd<Req<P>>,
                                   _: &platform::Snapshot<P>,
                                   _: &mut <P as PlatformTypes>::Effects)
                                   -> super::StepOutput<Addrd<Req<P>>, E>
    where P: PlatformTypes,
          Subs: Array<Item = Sub<P>>
  {
    match req.data().msg().observe() {
      | Some(Register) => {
        let mut sub = Some(Sub::new(req.clone()));
        self.subs
            .map_mut(move |s| s.push(Option::take(&mut sub).unwrap()));
      },
      | Some(Deregister) => {
        self.subs
            .map_mut(|s| match Self::get_index(s, req.data().msg().token) {
              | Some(ix) => {
                s.remove(ix);
              },
              | None => (),
            })
      },
      | _ => (),
    };

    Some(Ok(req))
  }

  fn clone_and_enqueue_sub_requests<P>(subs: &Subs, rq: &mut RequestQueue, path: &str)
    where P: PlatformTypes,
          Subs: Array<Item = Sub<P>>,
          RequestQueue: Array<Item = Addrd<Req<P>>>,
          Hasher: SubscriptionHash<P> + Default
  {
    Self::subs_matching_path(subs, path).for_each(|sub| {
                                          // TODO: handle option capacity
                                          let mut req = sub.req().clone();
                                          req.as_mut()
                                             .msg_mut()
                                             .set(opt::WAS_CREATED_BY_OBSERVE, Default::default())
                                             .ok();

                                          if rq.iter().all(|req2| {
                                                        Self::hash_req(&req) != Self::hash_req(req2)
                                                      })
                                          {
                                            rq.push(req);
                                          }
                                        });
  }
}

impl<P, S, B, RQ, H> Step<P> for Observe<S, B, RQ, H>
  where P: PlatformTypes,
        S: Step<P, PollReq = Addrd<Req<P>>, PollResp = Addrd<Resp<P>>>,
        B: Default + Array<Item = Sub<P>>,
        RQ: Default + Array<Item = Addrd<Req<P>>>,
        H: SubscriptionHash<P> + Default
{
  type PollReq = Addrd<Req<P>>;
  type PollResp = Addrd<Resp<P>>;

  type Error = S::Error;
  type Inner = S;

  fn inner(&self) -> &Self::Inner {
    &self.inner
  }

  fn poll_req(&self,
              snap: &platform::Snapshot<P>,
              effects: &mut <P as PlatformTypes>::Effects)
              -> super::StepOutput<Self::PollReq, Self::Error> {
    // TODO(orion): if throughput so high that there is always a request on the wire,
    // we will never fully flush the queue.
    // maybe add a timestamp or TTL check so that we can prioritize old outbound subscription updates
    match self.inner.poll_req(snap, effects) {
      | Some(Ok(req)) => self.handle_incoming_request(req, snap, effects),
      | None | Some(Err(nb::Error::WouldBlock)) => self.get_queued_request::<P>().map(Ok),
      | other => other,
    }
  }

  fn poll_resp(&self,
               snap: &platform::Snapshot<P>,
               effects: &mut <P as PlatformTypes>::Effects,
               token: ::toad_msg::Token,
               addr: no_std_net::SocketAddr)
               -> super::StepOutput<Self::PollResp, Self::Error> {
    self.inner.poll_resp(snap, effects, token, addr)
  }

  fn notify<Path>(&self, path: Path) -> Result<(), Self::Error>
    where Path: AsRef<str> + Clone
  {
    self.inner.notify(path.clone())?;

    self.request_queue.map_mut(|rq| {
                        Self::remove_queued_requests_matching_path(rq, path.as_ref());
                        self.subs.map_ref(|subs| {
                                   Self::clone_and_enqueue_sub_requests(subs, rq, path.as_ref())
                                 });
                      });

    Ok(())
  }

  fn before_message_sent(&self,
                         snap: &platform::Snapshot<P>,
                         effs: &mut P::Effects,
                         msg: &mut Addrd<platform::Message<P>>)
                         -> Result<(), Self::Error> {
    self.inner().before_message_sent(snap, effs, msg)?;

    if let Some(_) = msg.data().get(opt::WAS_CREATED_BY_OBSERVE) {
      msg.as_mut().remove(opt::WAS_CREATED_BY_OBSERVE);
    } else if msg.data().code.kind() == CodeKind::Response
              && self.subs
                     .map_ref(|subs| Self::get(subs, msg.addr(), msg.data().token).is_some())
    {
      self.subs.map_ref(|subs| {
                 Self::similar_to(subs, msg.addr(), msg.data().token).for_each(|sub| {
                   let mut msg = msg.clone();
                   msg.as_mut()
                      .set(opt::WAS_CREATED_BY_OBSERVE, Default::default())
                      .ok();
                   effs.push(Effect::Send(msg.with_addr(sub.addr())));
                 })
               });
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;
  use std::sync::Mutex;

  use ::toad_msg::{Code, ContentFormat, Id, Token, Type};
  use embedded_time::Clock;
  use lazycell::AtomicLazyCell;
  use platform::toad_msg;
  use tinyvec::array_vec;

  use super::*;
  use crate::platform::Effect;
  use crate::step::test::test_step;
  use crate::test;
  use crate::test::ClockMock;

  type Snapshot = crate::platform::Snapshot<test::Platform>;
  type Message = toad_msg::Message<test::Platform>;
  type Sub = super::Sub<test::Platform>;
  type Observe<S> = super::Observe<S,
                                   Vec<Sub>,
                                   Vec<Addrd<Req<test::Platform>>>,
                                   SubHash_TypePathQueryAccept<test::Platform>>;
  type PollReq = Addrd<Req<test::Platform>>;
  type PollResp = Addrd<Resp<test::Platform>>;

  test_step!(
      GIVEN Observe::<Dummy> where Dummy: {Step<PollReq = PollReq, PollResp = PollResp, Error = ()>};
      WHEN inner_errors [
        (inner.poll_req = { |_, _| Some(Err(nb::Error::Other(()))) }),
        (inner.poll_resp = { |_, _, _, _| Some(Err(nb::Error::Other(()))) })
      ]
      THEN this_should_error [
        (poll_req(_, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::Other(())))) }),
        (poll_resp(_, _, _, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::Other(())))) })
      ]
  );

  test_step!(
      GIVEN Observe::<Dummy> where Dummy: {Step<PollReq = PollReq, PollResp = PollResp, Error = ()>};
      WHEN inner_poll_req_oks [
        (inner.poll_resp = { |_, _, _, _| Some(Ok(Addrd(Resp::from(Message::new(Type::Con, Code::new(2, 4), Id(1), Token(Default::default()))), test::x.x.x.x(10)))) })
      ]
      THEN this_should_nop [
        (poll_resp(_, _, _, _) should satisfy { |out| assert_eq!(out, Some(Ok(Addrd(Resp::from(Message::new(Type::Con, Code::new(2, 4), Id(1), Token(Default::default()))), test::x.x.x.x(10))))) }),
        (effects should satisfy { |eff| assert!(eff.is_empty()) })
      ]
  );

  fn poll_req_emitting_single_register_request(
    num: usize)
    -> impl Fn(&Snapshot,
          &mut Vec<Effect<test::Platform>>)
          -> Option<nb::Result<Addrd<Req<test::Platform>>, ()>> {
    static INVOCATIONS: Mutex<AtomicLazyCell<HashMap<usize, usize>>> =
      Mutex::new(AtomicLazyCell::NONE);

    let lock = INVOCATIONS.lock().unwrap();
    if !lock.filled() {
      lock.fill(Default::default()).unwrap();
    }
    drop(lock);

    move |_, _| {
      let mut inv_lock = INVOCATIONS.lock().unwrap();
      let mut map = inv_lock.replace(Default::default()).unwrap();
      let call = *map.entry(num).and_modify(|n| *n += 1).or_insert(1);
      inv_lock.replace(map).unwrap();

      if call == 1 {
        let mut msg = test::msg!(CON GET x.x.x.x:80).unwrap();
        msg.id = Id(num as u16);
        msg.token = Token(array_vec!(num as u8));
        msg.set_path("foo/bar").ok();
        msg.set_observe(Register).ok();
        Some(Ok(Addrd(Req::from(msg), test::x.x.x.x(num as u16))))
      } else {
        None
      }
    }
  }

  test_step!(
      GIVEN Observe::<Dummy> where Dummy: {Step<PollReq = PollReq, PollResp = PollResp, Error = ()>};
      WHEN client_subscribes_and_event_fires [
        (inner.poll_req = { poll_req_emitting_single_register_request(11) }),
        ({|step: &Observe<Dummy>| {
          // Inner will yield a Register request,
          // this should add it to subscribtions list
          step.poll_req(&Snapshot { time: ClockMock::new().try_now().unwrap(),
                         recvd_dgram: None,
                         config: Default::default() }, &mut Default::default()).unwrap().unwrap()
        }}),
        // We have a new version available
        ({|step: &Observe<Dummy>| step.notify("foo/bar").unwrap()})
      ]
      THEN request_is_duplicated [
        // A copy of the original request should be emitted
        (poll_req(_, _) should satisfy { |req| {
          let req = req.unwrap().unwrap();
          assert_eq!(req.data().msg().token, Token(array_vec!(11)));
        }}),
        (poll_req(_, _) should satisfy { |req| assert!(req.is_none())  })
      ]
  );

  test_step!(
      GIVEN Observe::<Dummy> where Dummy: {Step<PollReq = PollReq, PollResp = PollResp, Error = ()>};
      WHEN response_to_subscriber_is_sent [
        // Store 2 subscriptions
        (inner.poll_req = { poll_req_emitting_single_register_request(21) }),
        ({|step: &Observe<Dummy>| step.poll_req(&Snapshot { time: ClockMock::new().try_now().unwrap(),
                         recvd_dgram: None,
                         config: Default::default() }, &mut Default::default()).unwrap().unwrap()}),
        (inner.poll_req = { poll_req_emitting_single_register_request(22) }),
        ({|step: &Observe<Dummy>| step.poll_req(&Snapshot { time: ClockMock::new().try_now().unwrap(),
                         recvd_dgram: None,
                         config: Default::default() }, &mut Default::default()).unwrap().unwrap()})
      ]
      THEN response_is_copied_and_sent_to_subscriber [
        (before_message_sent(_, _, test::msg!(CON { 2 . 05 } x.x.x.x:21 with |m: &mut Message<_, _>| {m.token = Token(array_vec!(21)); m.id = Id(1);})) should be ok with {|_| ()}),
        (effects should satisfy {|effs| {
          assert_eq!(effs.len(), 1);
          match effs.get(0).unwrap().clone() {
            platform::Effect::Send(m) => {
              assert_eq!(m.addr(), test::x.x.x.x(22));
              assert!(m.data().get(opt::WAS_CREATED_BY_OBSERVE).is_some());
            },
            _ => panic!(),
          }
        }})
      ]
  );

  test_step!(
      GIVEN Observe::<Dummy> where Dummy: {Step<PollReq = PollReq, PollResp = PollResp, Error = ()>};
      WHEN client_subscribes_and_unrelated_event_fires [
        (inner.poll_req = { poll_req_emitting_single_register_request(3) }),
        ({|step: &Observe<Dummy>| {
          step.poll_req(&Snapshot { time: test::ClockMock::new().try_now().unwrap(),
                         recvd_dgram: None,
                         config: crate::config::Config::default() }, &mut Default::default()).unwrap().unwrap()
        }}),
        ({|step: &Observe<Dummy>| step.notify("foot/bart").unwrap()})
      ]
      THEN nothing_happens [
        (poll_req(_, _) should satisfy { |req| assert!(req.is_none())  })
      ]
  );

  test_step!(
      GIVEN Observe::<Dummy> where Dummy: {Step<PollReq = PollReq, PollResp = PollResp, Error = ()>};
      WHEN client_subscribes_and_multiple_events_fire [
        (inner.poll_req = { poll_req_emitting_single_register_request(41) }),
        ({|step: &Observe<Dummy>| {
          step.poll_req(&Snapshot { time: test::ClockMock::new().try_now().unwrap(),
                         recvd_dgram: None,
                         config: crate::config::Config::default() }, &mut Default::default()).unwrap().unwrap()
        }}),
        ({|step: &Observe<Dummy>| step.notify("foo/bar").unwrap()}),
        ({|step: &Observe<Dummy>| {
          step.poll_req(&Snapshot { time: test::ClockMock::new().try_now().unwrap(),
                         recvd_dgram: None,
                         config: crate::config::Config::default() }, &mut Default::default()).unwrap().unwrap()
        }}),
        ({|step: &Observe<Dummy>| step.notify("foo/bar").unwrap()})
      ]
      THEN request_is_duplicated_multiple_times [
        (poll_req(_, _) should satisfy { |req| {
          let req = req.unwrap().unwrap();
          assert_eq!(req.data().msg().token, Token(array_vec!(41)));
        }}),
        (poll_req(_, _) should satisfy { |req| assert!(req.is_none())  })
      ]
  );

  #[test]
  pub fn sub_hash() {
    fn req<F>(stuff: F) -> u64
      where F: FnOnce(&mut Message)
    {
      let mut req = Message::new(Type::Con, Code::GET, Id(1), Token(Default::default()));
      stuff(&mut req);
      let sub = Sub::new(Addrd(Req::from(req), test::x.x.x.x(0)));

      let mut h = SubHash_TypePathQueryAccept::new();
      h.subscription_hash(sub.req());
      h.hasher().finish()
    }

    assert_ne!(req(|r| {
                 r.set_path("a/b/c").ok();
               }),
               req(|_| {}));
    assert_eq!(req(|r| {
                 r.set_path("a/b/c").ok();
               }),
               req(|r| {
                 r.set_path("a/b/c").ok();
               }));
    assert_ne!(req(|r| {
                 r.set_path("a/b/c").ok();
                 r.add_query("filter[temp](less_than)=123").ok();
               }),
               req(|r| {
                 r.set_path("a/b/c").ok();
               }));
    assert_eq!(req(|r| {
                 r.set_path("a/b/c").ok();
                 r.add_query("filter[temp](less_than)=123").ok();
               }),
               req(|r| {
                 r.set_path("a/b/c").ok();
                 r.add_query("filter[temp](less_than)=123").ok();
                 r.set_content_format(ContentFormat::Json).ok();
               }));
    assert_ne!(req(|r| {
                 r.set_path("a/b/c").ok();
                 r.add_query("filter[temp](less_than)=123").ok();
                 r.set_accept(ContentFormat::Json).ok();
               }),
               req(|r| {
                 r.set_path("a/b/c").ok();
                 r.add_query("filter[temp](less_than)=123").ok();
                 r.set_accept(ContentFormat::Text).ok();
               }));
    assert_eq!(req(|r| {
                 r.set_path("a/b/c").ok();
                 r.add_query("filter[temp](less_than)=123").ok();
                 r.set_accept(ContentFormat::Json).ok();
               }),
               req(|r| {
                 r.set_path("a/b/c").ok();
                 r.add_query("filter[temp](less_than)=123").ok();
                 r.set_accept(ContentFormat::Json).ok();
               }));
  }
}
