use core::marker::PhantomData;

use embedded_time::duration::Milliseconds;
use embedded_time::Instant;
use no_std_net::SocketAddr;
use toad_array::{AppendCopy, Array};
use toad_map::Map;
use toad_msg::no_repeat::{BLOCK1, BLOCK2, SIZE2};
use toad_msg::{CodeKind, Id, MessageOptions, Payload, Token, Type};
use toad_stem::Stem;

use super::{log, Step, _try};
use crate::net::Addrd;
use crate::platform::toad_msg::Message;
use crate::platform::{self, Effect, PlatformTypes, Snapshot};
use crate::req::Req;
use crate::resp::code::{CONTINUE, REQUEST_ENTITY_INCOMPLETE};
use crate::resp::Resp;

/// A potential role for a blocked message (request / response)
///
/// Part of composite map keys identifying [`Conversation`]s.
#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
pub enum Role {
  #[allow(missing_docs)]
  Request,
  #[allow(missing_docs)]
  Response,
}

/// Whether a blocked message is outbound or inbound
///
/// Part of composite map keys identifying [`Conversation`]s.
#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
pub enum Direction {
  #[allow(missing_docs)]
  Outbound,
  #[allow(missing_docs)]
  Inbound,
}

/// A single piece of a blocked message
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
pub enum Piece<M> {
  /// For [`Direction::Inbound`], indicates that we have received this piece,
  /// and have the data `M` associated with a piece.
  ///
  /// For [`Direction::Outbound`], indicates that we have not yet communicated
  /// this piece, and are waiting for the right time to transition to a different state.
  Data(M),
  /// For [`Direction::Inbound`], indicates that we have requested this piece,
  /// and expect that it will be transitioned to [`Piece::Have`] at some point in
  /// the future.
  ///
  /// For [`Direction::Outbound`], this state is impossible.
  Requested,
  /// For [`Direction::Inbound`], indicates that we have not yet received or asked for this piece.
  ///
  /// For [`Direction::Outbound`], indicates that we previously had this piece and have
  /// sent it to the remote endpoint.
  Absent,
}

impl<M> Piece<M> {
  fn get_msg(&self) -> Option<&M> {
    match self {
      | Piece::Data(m) => Some(m),
      | _ => None,
    }
  }
}

/// The state for a given Blocked conversation between ourselves and a remote endpoint.
#[derive(Debug)]
pub struct Conversation<P, Pieces>
  where P: PlatformTypes
{
  pub(self) biggest_number_seen: Option<u32>,
  pub(self) original: Option<Message<P>>,
  pub(self) pcs: Pieces,
  pub(self) expires_at: Instant<P::Clock>,
}

impl<P, Pieces> Conversation<P, Pieces>
  where P: PlatformTypes,
        Pieces: Default
{
  /// Create a new [`Conversation`] tracking a Blocked response to a sent request (param `msg`)
  pub fn new(expires_at: Instant<P::Clock>, original: Option<Message<P>>) -> Self {
    Self { original,
           biggest_number_seen: None,
           pcs: Default::default(),
           expires_at }
  }

  pub(self) fn assembled(&self) -> Payload<P::MessagePayload>
    where Pieces: Map<u32, Piece<Message<P>>>
  {
    const PANIC_MSG: &'static str = r#"BlockState.assembled() assumes:
- BlockState.biggest is Some(_)
- BlockState contains at least one Piece
- BlockState.have_all() has been invoked and confirmed to be true"#;

    let mut p = P::MessagePayload::default();
    for i in 0..=self.biggest_number_seen.expect(PANIC_MSG) {
      p.append_copy(&self.pcs
                         .get(&i)
                         .expect(PANIC_MSG)
                         .get_msg()
                         .expect(PANIC_MSG)
                         .payload
                         .0);
    }

    Payload(p)
  }

  /// find a missing piece that should be requested
  pub(self) fn get_missing<T>(&self) -> Option<u32>
    where Pieces: Map<u32, Piece<T>>,
          T: PartialEq
  {
    self.pcs
        .iter()
        .find_map(|(n, p)| if p == &Piece::Absent { Some(n) } else { None })
        .copied()
  }

  /// are no pieces waiting or missing?
  pub(self) fn have_all<T>(&self) -> bool
    where Pieces: Map<u32, Piece<T>>,
          T: PartialEq
  {
    self.pcs
        .iter()
        .all(|(_, p)| p != &Piece::Absent && p != &Piece::Requested)
  }

  /// if `n > self.biggest`, update `self.biggest` to `n`
  /// and insert `Piece::Missing` for all pieces between `biggest` and `n`
  pub(self) fn touch<T>(&mut self, now: Instant<P::Clock>, n: u32)
    where Pieces: Map<u32, Piece<T>>
  {
    let missing_nums = match self.biggest_number_seen {
      | Some(m) if m + 1 < n => (m + 1..n).into_iter(),
      | None if n > 0 => (0..n).into_iter(),
      | _ => (0..0).into_iter(),
    };

    missing_nums.for_each(|n| {
                  self.pcs.insert(n, Piece::Absent).ok();
                });

    let n_is_bigger = self.biggest_number_seen.map(|m| m < n).unwrap_or(true);
    if n_is_bigger {
      self.biggest_number_seen = Some(n);
    }
  }

  /// Mark piece `n` as having been requested
  pub(self) fn waiting<T>(&mut self, now: Instant<P::Clock>, n: u32)
    where Pieces: Map<u32, Piece<T>>
  {
    let e = self.pcs.get_mut(&n);

    match e {
      | Some(Piece::Absent) | Some(Piece::Requested) | None => {
        self.touch(now, n);
        self.pcs.insert(n, Piece::Requested).ok();
      },
      | _ => (),
    }
  }

  /// Store piece `T` corresponding to piece number `n`
  pub(self) fn have<T>(&mut self, now: Instant<P::Clock>, n: u32, m: T)
    where Pieces: Map<u32, Piece<T>>
  {
    self.touch(now, n);
    self.pcs.insert(n, Piece::Data(m)).ok();
  }
}

/// TODO
#[derive(Debug)]
pub struct Block<P, S, Endpoints, Conversations, Pieces> {
  inner: S,
  endpoints: Stem<Endpoints>,
  __p: PhantomData<(P, Conversations, Pieces)>,
}

impl<P, S, Endpoints, Conversations, Pieces> Default
  for Block<P, S, Endpoints, Conversations, Pieces>
  where S: Default,
        Endpoints: Default
{
  fn default() -> Self {
    Block { inner: S::default(),
            endpoints: Stem::new(Endpoints::default()),
            __p: PhantomData }
  }
}

impl<P, S, Endpoints, Conversations, Pieces> Block<P, S, Endpoints, Conversations, Pieces>
  where P: PlatformTypes,
        Endpoints: Default + Map<SocketAddr, Conversations>,
        Conversations:
          core::fmt::Debug + Default + Map<(Token, Role, Direction), Conversation<P, Pieces>>,
        Pieces: core::fmt::Debug + Default + Map<u32, Piece<Message<P>>>
{
  fn prune_conversations(&self,
                         effects: &mut P::Effects,
                         now: Instant<P::Clock>,
                         cs: &mut Conversations) {
    let len_before = cs.len();
    let mut remove_next = || {
      cs.iter()
        .filter(|(_, b)| now >= b.expires_at)
        .map(|(k, _)| *k)
        .next()
        .map(|k| cs.remove(&k))
        .map(|_| ())
    };

    while let Some(()) = remove_next() {}

    let len_after = cs.len();
    if len_before - len_after > 0 {
      log!(Block::prune, effects, log::Level::Debug, "Removed {} expired entries. For outbound messages, a prior step SHOULD but MAY NOT retry sending them", len_before - len_after);
    }
  }

  fn prune_endpoints(&self) {
    self.endpoints.map_mut(|es| {
                    let mut remove_next = || {
                      es.iter()
                        .filter(|(_, cs)| cs.is_empty())
                        .map(|(k, _)| *k)
                        .next()
                        .map(|k| es.remove(&k))
                        .map(|_| ())
                    };

                    while let Some(()) = remove_next() {}
                  });
  }

  fn prune(&self, effects: &mut P::Effects, now: Instant<P::Clock>) {
    // TODO: log
    self.endpoints.map_mut(|es| {
                    es.iter_mut()
                      .for_each(|(_, c)| self.prune_conversations(effects, now, c))
                  });
    self.prune_endpoints();
  }

  fn get_endpoint<F, R>(&self, addr: SocketAddr, mut f: F) -> Option<R>
    where F: FnMut(&mut Conversations) -> R
  {
    self.endpoints.map_mut(|es| es.get_mut(&addr).map(&mut f))
  }

  fn get_or_create_endpoint<F, R>(&self, addr: SocketAddr, mut f: F) -> R
    where F: FnMut(&mut Conversations) -> R
  {
    self.endpoints.map_mut(|es| {
                    if !es.has(&addr) {
                      es.insert(addr, Conversations::default()).unwrap();
                    }

                    es.get_mut(&addr).map(&mut f).unwrap()
                  })
  }

  fn insert(&self,
                    snap: &Snapshot<P>,
                    original: Option<&Message<P>>,
                    (addr, token, role, dir): (SocketAddr, Token, Role, Direction)) {
    let exp = snap.time + Milliseconds(snap.config.exchange_lifetime_millis());
    self.get_or_create_endpoint(addr, |convs| {
          convs.insert((token, role, dir), Conversation::new(exp, original.cloned()))
               .unwrap();
        });
  }

  fn has(&self, (addr, token, role, dir): (SocketAddr, Token, Role, Direction)) -> bool {
    self.get_endpoint(addr, |convs| convs.has(&(token, role, dir)))
        .unwrap_or(false)
  }

  fn map_mut<F, R>(&self,
                   (addr, token, role, dir): (SocketAddr, Token, Role, Direction),
                   mut f: F)
                   -> Option<R>
    where F: FnMut(&mut Conversation<P, Pieces>) -> R
  {
    self.get_or_create_endpoint(addr, |conv| conv.get_mut(&(token, role, dir)).map(&mut f))
  }

  fn map<F, R>(&self,
               (addr, token, role, dir): (SocketAddr, Token, Role, Direction),
               f: F)
               -> Option<R>
    where F: FnOnce(&Conversation<P, Pieces>) -> R
  {
    let mut f = Some(f);
    self.get_or_create_endpoint(addr, |conv| {
          conv.get(&(token, role, dir))
              .map(Option::take(&mut f).unwrap())
        })
  }

  fn remove_if_present(&self, (addr, token, role, dir): (SocketAddr, Token, Role, Direction)) {
    self.get_endpoint(addr, |convs| {
          convs.remove(&(token, role, dir));
        });
  }
}

impl<P, S, Endpoints, Conversations, Pieces> Step<P>
  for Block<P, S, Endpoints, Conversations, Pieces>
  where P: PlatformTypes,
        S: Step<P, PollReq = Addrd<Req<P>>, PollResp = Addrd<Resp<P>>>,
        Endpoints: core::fmt::Debug + Map<SocketAddr, Conversations>,
        Conversations: core::fmt::Debug + Map<(Token, Role, Direction), Conversation<P, Pieces>>,
        Pieces: core::fmt::Debug + Map<u32, Piece<Message<P>>>
{
  type PollReq = Addrd<Req<P>>;
  type PollResp = Addrd<Resp<P>>;
  type Error = S::Error;
  type Inner = S;

  fn inner(&self) -> &Self::Inner {
    &self.inner
  }

  fn poll_req(&self,
              snap: &crate::platform::Snapshot<P>,
              effects: &mut P::Effects)
              -> super::StepOutput<Self::PollReq, Self::Error> {
    self.prune(effects, snap.time);
    let mut req = _try!(Option<nb::Result>; self.inner().poll_req(snap, effects));

    macro_rules! respond {
      ($code:expr) => {{
        let rep_ty = match req.data().msg().ty {
          | Type::Con => Type::Ack,
          | _ => Type::Non,
        };

        let rep = Message::<P>::new(rep_ty, $code, Id(0), req.data().msg().token);
        effects.push(Effect::Send(Addrd(rep, req.addr())));
      }};
    }

    let k = (req.addr(), req.data().msg().token, Role::Request, Direction::Inbound);
    let has_prev_pieces = self.has(k);

    match req.data().msg().block2() {
      | None => {
        if has_prev_pieces {
          self.map(k, |s: &Conversation<_, _>| {
                log!(Block::poll_req,
                     effects,
                     log::Level::Warn,
                     "Expected message {:?} to continue block sequence {:?}",
                     req.data().msg().token,
                     s);
              });
          self.remove_if_present(k);
          respond!(REQUEST_ENTITY_INCOMPLETE);
        }

        Some(Ok(req))
      },
      | Some(block) => {
        if !has_prev_pieces && block.num() == 0 && block.more() {
          self.insert(snap, None, k);
          self.map_mut(k, |conv| {
                conv.have(snap.time, 0, req.clone().map(|r| r.into()).unwrap())
              });
          respond!(CONTINUE);
          Some(Err(nb::Error::WouldBlock))
        } else if !has_prev_pieces && block.num() == 0 && !block.more() {
          Some(Ok(req))
        } else if !has_prev_pieces && block.num() > 0 {
          respond!(REQUEST_ENTITY_INCOMPLETE);
          Some(Err(nb::Error::WouldBlock))
        } else if block.num()
                  > self.map(k, |conv| conv.biggest_number_seen.map(|n| n + 1))
                        .flatten()
                        .unwrap_or(0)
        {
          self.remove_if_present(k);
          respond!(REQUEST_ENTITY_INCOMPLETE);
          Some(Err(nb::Error::WouldBlock))
        } else if block.more() {
          self.map_mut(k, |conv| {
                conv.have(snap.time,
                          block.num(),
                          req.clone().map(|r| r.into()).unwrap())
              });
          respond!(CONTINUE);
          Some(Err(nb::Error::WouldBlock))
        } else {
          self.map_mut(k, |conv| {
                conv.have(snap.time,
                          block.num(),
                          req.clone().map(|r| r.into()).unwrap())
              });
          let p = self.map(k, Conversation::assembled).unwrap();
          req.as_mut().msg_mut().payload = p;
          self.remove_if_present(k);
          Some(Ok(req))
        }
      },
    }
  }

  fn poll_resp(&self,
               snap: &Snapshot<P>,
               effects: &mut P::Effects,
               token: Token,
               addr: SocketAddr)
               -> super::StepOutput<Self::PollResp, Self::Error> {
    self.prune(effects, snap.time);

    let mut rep: Addrd<Resp<P>> =
      _try!(Option<nb::Result>; self.inner().poll_resp(snap, effects, token, addr));

    let k = (rep.addr(), rep.data().msg().token, Role::Response, Direction::Inbound);
    let has_prev_pieces = self.has(k);

    macro_rules! request_piece {
      ($num:expr) => {{
        let mut new = self.map(k, |conv| conv.original.clone().unwrap()).unwrap();

        new.set_block1(0, $num, false).ok();
        new.remove(BLOCK2);
        new.remove(SIZE2);

        effects.push(Effect::Send(Addrd(new, rep.addr())));
        self.map_mut(k, |conv| conv.waiting(snap.time, $num));
      }};
    }

    match rep.data().msg().block1() {
      | None => {
        self.remove_if_present(k);
        Some(Ok(rep))
      },
      | Some(block) => {
        if !has_prev_pieces {
          log!(Block::poll_resp, effects, log::Level::Warn, "Response received for token {:?} but we've never seen a request using that token. Ignoring this response despite it having {:?}", rep.data().msg().token, block);
          Some(Ok(rep))
        } else {
          self.map_mut(k, |conv| {
                conv.have(snap.time,
                          block.num(),
                          rep.clone().map(|r| r.into()).unwrap())
              });
          if block.more() {
            request_piece!(block.num() + 1);
          }

          if let Some(missing) = self.map(k, Conversation::get_missing).unwrap() {
            request_piece!(missing);
          }

          if self.map(k, Conversation::have_all).unwrap() {
            rep.as_mut().msg_mut().payload = self.map(k, Conversation::assembled).unwrap();
            rep.as_mut().msg_mut().remove(BLOCK1);
            self.remove_if_present(k);
            Some(Ok(rep))
          } else {
            Some(Err(nb::Error::WouldBlock))
          }
        }
      },
    }
  }

  fn before_message_sent(&self,
                     snap: &platform::Snapshot<P>,
                     effs: &mut P::Effects,
                     msg: &mut Addrd<Message<P>>)
                     -> Result<(), Self::Error> {
    self.prune(effs, snap.time);
    self.inner.before_message_sent(snap, effs, msg)?;

    let block_size: usize = 1024;

    let original_payload = msg.data().payload().0;

    // TODO: block if 1024 is too big and we got REQUEST_ENTITY_TOO_LARGE
    if msg.data().block1().is_none() && original_payload.len() > block_size {
        let k = (msg.addr(), msg.data().token, Role::Request, Direction::Outbound);
      self.insert(snap, Some(msg.data()), k);
      self.map_mut(k, |conv| {
        let len = original_payload.len() as f32;
        let block_count = (len / block_size as f32).ceil() as u32;
        for n in 0..block_count {
          let mut msg_block = msg.clone();
          msg_block.as_mut().set_block1(1024, n, n == block_count - 1).ok();
          let mut p = P::MessagePayload::default();
          p.append_copy(original_payload[n * 1024..((n + 1) * 1024)]);
          msg_block.as_mut().payload = Payload(p);
          conv.have(snap.time, n, msg_block.unwrap());
        }
      }).unwrap();
    }
    Ok(())
  }

  fn on_message_sent(&self,
                     snap: &platform::Snapshot<P>,
                     effs: &mut P::Effects,
                     msg: &Addrd<Message<P>>)
                     -> Result<(), Self::Error> {
    self.inner.on_message_sent(snap, effs, msg)?;
    if msg.data().code.kind() == CodeKind::Request {
      self.insert(snap, Some(msg.data()), (msg.addr(), msg.data().token, Role::Response, Direction::Inbound));
    } else if msg.data().code.kind() == CodeKind::Response {
      // TODO: block outbound responses
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use std_alloc::collections::BTreeMap;
  use tinyvec::array_vec;
  use toad_msg::{Code, ContentFormat, Id, MessageOptions, Type};

  use super::*;
  use crate::net::Addrd;
  use crate::resp::code::CONTENT;
  use crate::test;

  type Pieces = BTreeMap<u32, Piece<test::Message>>;
  type Conversations =
    BTreeMap<(Token, Role, Direction), super::Conversation<test::Platform, Pieces>>;
  type Block<S> =
    super::Block<test::Platform, S, BTreeMap<SocketAddr, Conversations>, Conversations, Pieces>;

  #[test]
  fn block_state_correctly_identifies_missing_pieces() {
    let mut e =
      Conversation::<test::Platform, BTreeMap<u32, Piece<()>>> { biggest_number_seen: None,
                                                                 original: None,
                                                                 pcs: BTreeMap::new(),
                                                                 expires_at: Instant::new(1000) };
    e.have(Instant::new(0), 0, ());
    assert_eq!(e.get_missing(), None);

    e.have(Instant::new(0), 1, ());
    assert_eq!(e.get_missing(), None);

    e.waiting(Instant::new(0), 3);
    e.waiting(Instant::new(0), 2);
    e.waiting(Instant::new(0), 5);

    assert_eq!(e.get_missing(), Some(4));
    e.waiting(Instant::new(0), 4);

    assert_eq!(e.get_missing(), None);
  }

  #[test]
  fn when_inner_errors_block_should_error() {
    type S = test::MockStep<(), Addrd<test::Req>, Addrd<test::Resp>, ()>;
    let b = Block::<S>::default();

    b.inner()
     .set_poll_req(|_, _, _| Some(Err(nb::Error::Other(()))))
     .set_poll_resp(|_, _, _, _, _| Some(Err(nb::Error::Other(()))));

    assert_eq!(b.poll_req(&test::snapshot(), &mut vec![]),
               Some(Err(nb::Error::Other(()))));
    assert_eq!(b.poll_resp(&test::snapshot(),
                           &mut vec![],
                           Token(Default::default()),
                           test::x.x.x.x(80)),
               Some(Err(nb::Error::Other(()))));
  }

  #[test]
  fn when_recv_response_with_no_block1_this_should_do_nothing() {
    type S = test::MockStep<(), Addrd<test::Req>, Addrd<test::Resp>, ()>;
    let b = Block::<S>::default();

    b.inner().set_poll_resp(|_, _, _, _, _| {
               let msg = toad_msg::alloc::Message::new(Type::Con,
                                                       Code::GET,
                                                       Id(0),
                                                       Token(Default::default()));
               Some(Ok(Addrd(Resp::from(msg), test::x.x.x.x(80))))
             });

    let mut effects = vec![];
    assert!(matches!(b.poll_resp(&test::snapshot(),
                                 &mut effects,
                                 Token(Default::default()),
                                 test::x.x.x.x(80)),
                     Some(Ok(Addrd(_, _)))));
    assert!(effects.is_empty());
  }

  #[test]
  fn when_recv_response_with_block1_this_should_ask_for_other_blocks() {
    struct TestState {
      gave_pieces: Vec<u32>,
      req: Option<Addrd<test::Req>>,
      last_request_at: std::time::Instant,
    }

    #[allow(dead_code)]
    struct Addrs {
      server: SocketAddr,
      client: SocketAddr,
    }

    let addrs = Addrs { server: test::x.x.x.x(80),
                        client: test::x.x.x.x(10) };

    type S = test::MockStep<TestState, Addrd<test::Req>, Addrd<test::Resp>, ()>;
    let b = Block::<S>::default();

    let mut orig_req = test::Message::new(Type::Con, Code::GET, Id(1), Token(array_vec! {1}));
    orig_req.set_accept(ContentFormat::Text).ok();
    orig_req.set_path("lipsum").ok();

    let cache_key = orig_req.cache_key();

    let mut effects: Vec<test::Effect> = vec![];

    b.on_message_sent(&test::snapshot(),
                      &mut effects,
                      &Addrd(orig_req, addrs.server))
     .unwrap();

    let payload = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do";
    let payload_blocks = || {
      payload.bytes().fold(vec![vec![]], |mut b, a| {
                       let last = b.last_mut().unwrap();
                       if last.len() < 16 {
                         last.push(a);
                       } else {
                         b.push(vec![a]);
                       }
                       b
                     })
    };

    b.inner()
     .init(TestState { gave_pieces: vec![],
                       req: None,
                       last_request_at: std::time::Instant::now() })
     .set_poll_resp(move |mock, _, _, _, _| {
       let blocksize: u16 = 16;
       let blocks = payload_blocks();

       let mut resp =
         Resp::from(test::Message::new(Type::Non, Code::new(2, 05), Id(1), Token(array_vec! {1})));
       resp.msg_mut().set_size1(payload.len() as _).ok();
       resp.msg_mut().set_path("lipsum").ok();

       let request = mock.state.map_ref(|s| s.as_ref().unwrap().req.clone());

       let requested_piece = request.as_ref()
                                    .and_then(|req| req.data().msg().block1())
                                    .map(|b| b.num());
       let already_gave_pieces = mock.state
                                     .map_ref(|s| s.as_ref().unwrap().gave_pieces.clone());
       let last_request_at = mock.state
                                 .map_ref(|s| s.as_ref().unwrap().last_request_at.clone());
       let elapsed = std::time::Instant::now().duration_since(last_request_at);

       match requested_piece {
         | None if already_gave_pieces.is_empty() => {
           resp.set_payload(blocks[0].iter().copied());
           resp.msg_mut().set_block1(blocksize, 0, true).ok();
           mock.state
               .map_mut(|s| s.as_mut().unwrap().last_request_at = std::time::Instant::now());
         },
         | None if request.is_none() && elapsed > std::time::Duration::from_secs(1) => {
           panic!("timeout")
         },
         | None => panic!("Block1 not set on request when client already got a Block1 response"),
         | Some(_) if request.map(|r| r.data().msg().cache_key()) != Some(cache_key) => {
           panic!("cache_key mismatch!")
         },
         | Some(n)
           if already_gave_pieces.iter()
                                 .any(|p| Some(*p) == requested_piece) =>
         {
           panic!("block {n} already given")
         },
         | Some(n) if n > 3 => panic!("block {n} out of range"),
         | Some(n) => {
           resp.set_payload(blocks[n as usize].iter().copied());
           resp.msg_mut().set_block1(blocksize, n, n < 3).ok();
           mock.state.map_mut(|s| {
                       let s = s.as_mut().unwrap();
                       s.gave_pieces.push(n);
                       s.last_request_at = std::time::Instant::now();
                     });
         },
       }

       Some(Ok(Addrd(resp, addrs.server)))
     });

    let rep = loop {
      let mut reqs = effects.drain(..)
                            .filter(|e| e.is_send())
                            .collect::<Vec<_>>();
      match reqs.len() {
        | 0 => (),
        | 1 => {
          let mut req = reqs.remove(0)
                            .get_send()
                            .cloned()
                            .map(|addrd| addrd.map(Req::from));
          b.inner()
           .state
           .map_mut(|s| s.as_mut().unwrap().req = Option::take(&mut req));
        },
        | _ => panic!("too many outbound messages queued ({:?})",
                      reqs.iter()
                          .cloned()
                          .map(|r| r.get_send()
                                    .as_ref()
                                    .unwrap()
                                    .data()
                                    .block1()
                                    .unwrap()
                                    .num())
                          .collect::<Vec<_>>()),
      }

      match b.poll_resp(&test::snapshot(),
                        &mut effects,
                        Token(array_vec! {1}),
                        addrs.server)
      {
        | Some(Err(nb::Error::WouldBlock)) => continue,
        | Some(Err(nb::Error::Other(e))) => panic!("{e:?}"),
        | Some(Ok(rep)) => break rep,
        | None => panic!("got None"),
      }
    };

    assert_eq!(rep.data().payload().copied().collect::<Vec<u8>>(),
               payload.bytes().collect::<Vec<u8>>());
  }

  #[test]
  fn when_recv_request_with_block2_and_dont_hear_another_for_a_long_time_this_should_prune_state(
    ) {
    #[derive(Clone, Copy)]
    #[allow(dead_code)]
    struct Addrs {
      server: SocketAddr,
      client: SocketAddr,
    }

    let addrs = Addrs { server: test::x.x.x.x(80),
                        client: test::x.x.x.x(10) };
    let addrs: &'static Addrs = unsafe { core::mem::transmute(&addrs) };

    type S = test::MockStep<(), Addrd<test::Req>, Addrd<test::Resp>, ()>;
    let b = Block::<S>::default();

    b.inner().set_poll_req(|_, snap, _| {
               if snap.time == Instant::new(0) {
                 let mut req =
                   test::Message::new(Type::Non, Code::GET, Id(0), Token(Default::default()));
                 req.set_block2(128, 0, true).ok();
                 Some(Ok(Addrd(Req::from(req), addrs.client)))
               } else {
                 None
               }
             });

    let t_0 = test::snapshot();
    let mut t_1 = test::snapshot();
    t_1.time = Instant::new(0) + Milliseconds(t_1.config.exchange_lifetime_millis() - 1);

    let mut t_2 = test::snapshot();
    t_2.time = Instant::new(0) + Milliseconds(t_2.config.exchange_lifetime_millis() + 1);

    assert!(matches!(b.poll_req(&t_0, &mut vec![]).unwrap().unwrap_err(),
                     nb::Error::WouldBlock));
    assert_eq!(b.get_endpoint(addrs.client, |convs| convs.len()).unwrap(),
               1);

    assert!(matches!(b.poll_req(&t_1, &mut vec![]), None));
    assert_eq!(b.get_endpoint(addrs.client, |convs| convs.len()).unwrap(),
               1);

    assert!(matches!(b.poll_req(&t_2, &mut vec![]), None));
    assert_eq!(b.get_endpoint(addrs.client, |convs| convs.len()), None);
  }

  #[test]
  fn when_recv_response_with_block2_and_dont_hear_back_for_a_long_time_this_should_prune_state() {
    #[derive(Clone, Copy)]
    #[allow(dead_code)]
    struct Addrs {
      server: SocketAddr,
      client: SocketAddr,
    }

    let addrs = Addrs { server: test::x.x.x.x(80),
                        client: test::x.x.x.x(10) };
    let addrs: &'static Addrs = unsafe { core::mem::transmute(&addrs) };

    type S = test::MockStep<(), Addrd<test::Req>, Addrd<test::Resp>, ()>;
    let b = Block::<S>::default();

    b.inner().set_poll_resp(|_, snap, _, _, _| {
               if snap.time == Instant::new(0) {
                 let mut rep =
                   test::Message::new(Type::Con, CONTENT, Id(0), Token(Default::default()));
                 rep.set_block1(128, 0, true).ok();
                 Some(Ok(Addrd(Resp::from(rep), addrs.server)))
               } else {
                 None
               }
             });

    let mut effects = vec![];
    let t_0 = test::snapshot();
    let mut t_n1 = test::snapshot();
    t_n1.time = Instant::new(0) + Milliseconds(t_n1.config.exchange_lifetime_millis() - 1);

    let mut t_n2 = test::snapshot();
    t_n2.time = Instant::new(0) + Milliseconds(t_n2.config.exchange_lifetime_millis() + 1);

    let req = test::Message::new(Type::Non, Code::GET, Id(0), Token(Default::default()));
    let req = Addrd(req, addrs.server);

    b.on_message_sent(&t_0, &mut effects, &req).unwrap();

    let rep_0 = b.poll_resp(&t_0, &mut effects, Token(Default::default()), addrs.server)
                 .unwrap()
                 .unwrap_err();

    assert!(matches!(rep_0, nb::Error::WouldBlock));

    assert_eq!(b.get_endpoint(addrs.server, |convs| convs.len()).unwrap(),
               1);

    b.poll_resp(&t_n1, &mut effects, Token(Default::default()), addrs.server)
     .ok_or(())
     .unwrap_err();

    assert_eq!(b.get_endpoint(addrs.server, |convs| convs.len()).unwrap(),
               1);

    b.poll_resp(&t_n2, &mut effects, Token(Default::default()), addrs.server)
     .ok_or(())
     .unwrap_err();

    assert_eq!(b.get_endpoint(addrs.server, |convs| convs.len()), None);
  }

  #[test]
  fn when_recv_request_without_block2_this_should_do_nothing() {
    #[derive(Clone, Copy)]
    #[allow(dead_code)]
    struct Addrs {
      server: SocketAddr,
      client: SocketAddr,
    }

    let addrs = Addrs { server: test::x.x.x.x(80),
                        client: test::x.x.x.x(10) };
    let addrs: &'static Addrs = unsafe { core::mem::transmute(&addrs) };

    type S = test::MockStep<(), Addrd<test::Req>, Addrd<test::Resp>, ()>;
    let b = Block::<S>::default();

    b.inner().set_poll_req(|_, _, _| {
               let req =
                 test::Message::new(Type::Con, Code::POST, Id(0), Token(Default::default()));
               Some(Ok(Addrd(Req::from(req), addrs.client)))
             });

    let mut effects = vec![];
    b.poll_req(&test::snapshot(), &mut effects)
     .unwrap()
     .unwrap();

    assert!(effects.is_empty());
  }

  #[test]
  fn when_recv_request_with_block2_and_recognized_number_this_should_respond_2_31() {
    struct TestState {
      next_block: u32,
    }

    #[derive(Clone, Copy)]
    #[allow(dead_code)]
    struct Addrs {
      server: SocketAddr,
      client: SocketAddr,
    }

    let addrs = Addrs { server: test::x.x.x.x(80),
                        client: test::x.x.x.x(10) };
    let addrs: &'static Addrs = unsafe { core::mem::transmute(&addrs) };

    type S = test::MockStep<TestState, Addrd<test::Req>, Addrd<test::Resp>, ()>;
    let b = Block::<S>::default();

    b.inner()
     .init(TestState { next_block: 0 })
     .set_poll_req(|mock, _, _| {
       let mut req = test::Message::new(Type::Con, Code::POST, Id(0), Token(Default::default()));
       let num = mock.state.map_ref(|s| s.as_ref().unwrap().next_block);
       req.set_block2(128, num, num < 2).ok();
       req.set_payload(Payload(core::iter::repeat(0u8).take(128).collect()));

       mock.state.map_mut(|s| s.as_mut().unwrap().next_block += 1);
       Some(Ok(Addrd(Req::from(req), addrs.client)))
     });

    let mut effects = vec![];

    // get block 0
    assert_eq!(b.poll_req(&test::snapshot(), &mut effects),
               Some(Err(nb::Error::WouldBlock)));

    let resp = effects[0].get_send().unwrap();
    assert_eq!(resp.data().code, Code::new(2, 31));
    effects.clear();

    // get block 1
    assert_eq!(b.poll_req(&test::snapshot(), &mut effects),
               Some(Err(nb::Error::WouldBlock)));

    let resp = effects[0].get_send().unwrap();
    assert_eq!(resp.data().code, Code::new(2, 31));
    effects.clear();

    // get block 2
    let assembled = b.poll_req(&test::snapshot(), &mut effects);
    assert!(matches!(assembled, Some(Ok(_))));
    assert_eq!(assembled.unwrap().unwrap().data().payload().len(), 128 * 3);
    assert!(effects.is_empty());
  }

  #[test]
  fn when_recv_request_with_block2_and_unrecognized_number_this_should_respond_4_08() {
    #[derive(Clone, Copy)]
    #[allow(dead_code)]
    struct Addrs {
      server: SocketAddr,
      client: SocketAddr,
    }

    let addrs = Addrs { server: test::x.x.x.x(80),
                        client: test::x.x.x.x(10) };
    let addrs: &'static Addrs = unsafe { core::mem::transmute(&addrs) };

    type S = test::MockStep<(), Addrd<test::Req>, Addrd<test::Resp>, ()>;
    let b = Block::<S>::default();

    b.inner().set_poll_req(|_, _, _| {
               let mut req =
                 test::Message::new(Type::Con, Code::POST, Id(0), Token(Default::default()));
               req.set_block2(128, 1, true).ok();
               req.set_payload(Payload(core::iter::repeat(0u8).take(128).collect()));
               Some(Ok(Addrd(Req::from(req), addrs.client)))
             });

    let mut effects = vec![];
    assert_eq!(b.poll_req(&test::snapshot(), &mut effects),
               Some(Err(nb::Error::WouldBlock)));

    assert!(!effects.is_empty());

    let resp = effects[0].get_send().unwrap();
    assert_eq!(resp.data().code, Code::new(4, 08));
  }
}
