use core::fmt::Debug;
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;

use naan::prelude::{Apply, F2Once};
use toad_common::hash::Blake2Hasher;
use toad_common::{Array, Stem};
use toad_msg::opt::known::observe::Action::{Deregister, Register};
use toad_msg::opt::known::repeat::QUERY;
use toad_msg::{Id, MessageOptions, OptValue, Token};

use super::{Step, _try};
use crate::net::Addrd;
use crate::platform::{self, PlatformTypes};
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

  fn subscription_hash(&mut self, sub: &Sub<P>) {
    let h = SubscriptionHash::<P>::hasher(self);
    let msg = sub.req().data().msg();

    msg.ty.hash(h);
    msg.get(QUERY).into_iter().for_each(|v| {
                                v.hash(h);
                              });
    msg.accept().hash(h);
    msg.path().ok().hash(h);
  }
}

/// Get a hash used to determine whether similar subscriptions
/// may be grouped together.
///
/// When your server [`notify`](super::Step::notify)s the toad runtime
/// that there is a new version of a resource available, all
/// subscriptions matching the path passed to `notify` will be
/// re-sent as new requests to your server.
///
/// Similar requests (determined by this trait) will be grouped together
/// so that your server only sees 1 request, and the response
/// will be fanned back out to the subscribers.
///
/// A default implementation is provided by [`SubHash_TypePathQueryAccept`].
pub trait SubscriptionHash<P>
  where Self: Sized + Debug,
        P: PlatformTypes
{
  /// Type used to generate hashes
  type Hasher: Hasher;

  #[allow(missing_docs)]
  fn hasher(&mut self) -> &mut Self::Hasher;

  #[allow(missing_docs)]
  fn subscription_hash(&mut self, sub: &Sub<P>);
}

impl<P, T> SubscriptionHash<P> for &mut T
  where P: PlatformTypes,
        T: SubscriptionHash<P>
{
  type Hasher = T::Hasher;

  fn hasher(&mut self) -> &mut Self::Hasher {
    <T as SubscriptionHash<P>>::hasher(self)
  }

  fn subscription_hash(&mut self, sub: &Sub<P>) {
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
  /// TODO
  pub fn hash<'a, P>(sub: &'a Sub<P>) -> (&'a Sub<P>, u64)
    where P: PlatformTypes,
          Hasher: SubscriptionHash<P> + Default
  {
    let mut h = Hasher::default();
    h.subscription_hash(sub);
    (sub, h.hasher().finish())
  }

  /// TODO
  pub fn get<'a, P>(subs: &'a Subs, t: Token) -> Option<&'a Sub<P>>
    where Subs: Array<Item = Sub<P>>,
          P: PlatformTypes
  {
    subs.iter().find(|s| s.token() == t)
  }

  /// TODO
  pub fn get_index<'a, P>(subs: &'a Subs, t: Token) -> Option<usize>
    where Subs: Array<Item = Sub<P>>,
          P: PlatformTypes
  {
    subs.iter()
        .enumerate()
        .find(|(_, s)| s.token() == t)
        .map(|(ix, _)| ix)
  }

  /// TODO
  pub fn similar_to<'a, P>(subs: &'a Subs, t: Token) -> impl 'a + Iterator<Item = &'a Sub<P>>
    where Subs: Array<Item = Sub<P>>,
          P: PlatformTypes,
          Hasher: SubscriptionHash<P> + Default
  {
    subs.iter()
        .filter(move |s| match Self::get(subs, t).map(Self::hash) {
          | Some((sub, h)) => s.id() != sub.id() && Self::hash(sub).1 == h,
          | None => false,
        })
  }

  /// TODO
  pub fn subs_matching_path<'a, 'b, P>(subs: &'a Subs,
                                       p: &'b str)
                                       -> impl 'a + Iterator<Item = &'a Sub<P>>
    where Subs: Array<Item = Sub<P>>,
          P: PlatformTypes,
          'b: 'a
  {
    subs.iter()
        .filter(move |s| s.msg().path().ok().flatten().unwrap_or("") == p)
  }

  fn remove_queued_requests_matching_path<P>(rq: &mut RequestQueue, path: &str) -> ()
    where P: PlatformTypes,
          RequestQueue: Array<Item = Addrd<Req<P>>>
  {
    fn go<P, RQ>(rq: &mut RQ, path: &str) -> ()
      where P: PlatformTypes,
            RQ: Array<Item = Addrd<Req<P>>>
    {
      match rq.iter()
              .enumerate()
              .find(|(_, req)| req.data().msg().path().ok().flatten() == Some(path))
              .map(|(ix, _)| ix)
      {
        | Some(ix) => {
          rq.remove(ix);
          go(rq, path);
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
          RequestQueue: Array<Item = Addrd<Req<P>>>
  {
    Self::subs_matching_path(subs, path).for_each(|sub| {
                                          // TODO: handle option capacity
                                          let mut req = sub.req().clone();
                                          req.as_mut()
                                             .msg_mut()
                                             .set(opt::WAS_CREATED_BY_OBSERVE, Default::default())
                                             .ok();
                                          rq.push(req);
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
                        // TODO: dedup request_queue using hash
                      });

    Ok(())
  }

  fn before_message_sent(&self,
                         snap: &platform::Snapshot<P>,
                         msg: &mut Addrd<platform::Message<P>>)
                         -> Result<(), Self::Error> {
    // FAN OUT
    // [0] WAS_CREATED_BY_OBSERVE? if so do NOT process, strip option and continue
    // [1] is response?
    // [2] self.has(token)?
    // [3] self.similar_to <#> copy response (with WAS_CREATED_BY_OBSERVE)
    // [4] effects.push(<send response>)
    todo!()
  }
}

#[cfg(test)]
mod tests {
  use core::mem::MaybeUninit;
  use std::collections::HashMap;
  use std::sync::Mutex;

  use ::toad_msg::{Code, ContentFormat, Id, Token, Type};
  use embedded_time::Clock;
  use lazycell::{AtomicLazyCell, LazyCell};
  use platform::toad_msg;
  use tinyvec::array_vec;

  use super::*;
  use crate::platform::Effect;
  use crate::step::test::test_step;
  use crate::test;

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
    test_id: usize)
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
      let call = *map.entry(test_id).and_modify(|n| *n += 1).or_insert(1);
      inv_lock.replace(map).unwrap();

      if call == 1 {
        let mut msg = test::msg!(CON GET x.x.x.x:80);
        msg.as_mut().token = Token(array_vec!(1, 2, 3, 4));
        msg.as_mut().set_path("foo/bar").ok();
        msg.as_mut().set_observe(Register).ok();
        Some(Ok(msg.map(Req::from)))
      } else {
        None
      }
    }
  }

  test_step!(
      GIVEN Observe::<Dummy> where Dummy: {Step<PollReq = PollReq, PollResp = PollResp, Error = ()>};
      WHEN client_subscribes_and_event_fires [
        (inner.poll_req = { poll_req_emitting_single_register_request(1) }),
        ({|step: &Observe<Dummy>| {
          // Inner will yield a Register request,
          // this should add it to subscribtions list
          step.poll_req(&Snapshot { time: test::ClockMock::new().try_now().unwrap(),
                         recvd_dgram: None,
                         config: crate::config::Config::default() }, &mut Default::default()).unwrap().unwrap()
        }}),
        // We have a new version available
        ({|step: &Observe<Dummy>| step.notify("foo/bar").unwrap()})
      ]
      THEN request_is_duplicated [
        // A copy of the original request should be emitted
        (poll_req(_, _) should satisfy { |req| {
          let req = req.unwrap().unwrap();
          assert_eq!(req.data().msg().token, Token(array_vec!(1, 2, 3, 4)));
        }}),
        (poll_req(_, _) should satisfy { |req| assert!(req.is_none())  })
      ]
  );

  test_step!(
      GIVEN Observe::<Dummy> where Dummy: {Step<PollReq = PollReq, PollResp = PollResp, Error = ()>};
      WHEN client_subscribes_and_unrelated_event_fires [
        (inner.poll_req = { poll_req_emitting_single_register_request(2) }),
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
        (inner.poll_req = { poll_req_emitting_single_register_request(3) }),
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
          assert_eq!(req.data().msg().token, Token(array_vec!(1, 2, 3, 4)));
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
      h.subscription_hash(&sub);
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
