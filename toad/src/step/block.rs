use core::marker::PhantomData;

use embedded_time::duration::Milliseconds;
use embedded_time::Instant;
use naan::prelude::F1Once;
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

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
enum Piece<M> {
  Have(M),
  Waiting,
  Missing,
}

impl<M> Piece<M> {
  fn get_msg(&self) -> Option<&M> {
    match self {
      | Piece::Have(m) => Some(m),
      | _ => None,
    }
  }
}

#[derive(Debug)]
struct BlockState<P, Pcs>
  where P: PlatformTypes
{
  pub(self) biggest: Option<u32>,
  pub(self) original: Option<Addrd<Message<P>>>,
  pub(self) pcs: Pcs,
  pub(self) expires_at: Instant<P::Clock>,
}

impl<P, Pcs> BlockState<P, Pcs> where P: PlatformTypes
{
  pub(self) fn assembled(&self) -> Payload<P::MessagePayload>
    where Pcs: Map<u32, Piece<Addrd<Message<P>>>>
  {
    const PANIC_MSG: &'static str = r#"BlockState.assembled() assumes:
- BlockState.biggest is Some(_)
- BlockState contains at least one Piece
- BlockState.have_all() has been invoked and confirmed to be true"#;

    let mut p = P::MessagePayload::default();
    for i in 0..=self.biggest.expect(PANIC_MSG) {
      p.append_copy(&self.pcs
                         .get(&i)
                         .expect(PANIC_MSG)
                         .get_msg()
                         .expect(PANIC_MSG)
                         .data()
                         .payload
                         .0);
    }

    Payload(p)
  }

  /// find a missing piece that should be requested
  pub(self) fn get_missing<T>(&self) -> Option<u32>
    where Pcs: Map<u32, Piece<T>>,
          T: PartialEq
  {
    self.pcs
        .iter()
        .find_map(|(n, p)| if p == &Piece::Missing { Some(n) } else { None })
        .copied()
  }

  /// are no pieces waiting or missing?
  pub(self) fn have_all<T>(&self) -> bool
    where Pcs: Map<u32, Piece<T>>,
          T: PartialEq
  {
    self.pcs
        .iter()
        .all(|(_, p)| p != &Piece::Missing && p != &Piece::Waiting)
  }

  /// if `n > self.biggest`, update `self.biggest` to `n`
  /// and insert `Piece::Missing` for all pieces between `biggest` and `n`
  pub(self) fn touch<T>(&mut self, now: Instant<P::Clock>, n: u32)
    where Pcs: Map<u32, Piece<T>>
  {
    let missing_nums = match self.biggest {
      | Some(m) if m + 1 < n => (m + 1..n).into_iter(),
      | None if n > 0 => (0..n).into_iter(),
      | _ => (0..0).into_iter(),
    };

    missing_nums.for_each(|n| {
                  self.pcs.insert(n, Piece::Missing).ok();
                });

    let n_is_bigger = self.biggest.map(|m| m < n).unwrap_or(true);
    if n_is_bigger {
      self.biggest = Some(n);
    }
  }

  /// Mark piece `n` as having been requested
  pub(self) fn waiting<T>(&mut self, now: Instant<P::Clock>, n: u32)
    where Pcs: Map<u32, Piece<T>>
  {
    let e = self.pcs.get_mut(&n);

    match e {
      | Some(Piece::Missing) | Some(Piece::Waiting) | None => {
        self.touch(now, n);
        self.pcs.insert(n, Piece::Waiting).ok();
      },
      | _ => (),
    }
  }

  /// Store piece `T` corresponding to piece number `n`
  pub(self) fn have<T>(&mut self, now: Instant<P::Clock>, n: u32, m: T)
    where Pcs: Map<u32, Piece<T>>
  {
    self.touch(now, n);
    self.pcs.insert(n, Piece::Have(m)).ok();
  }

  pub(self) fn biggest(&self) -> Option<u32> {
    self.biggest
  }
}

/// TODO
#[derive(Debug)]
pub struct Block<P, S, BS, Pcs> {
  inner: S,
  block_states: Stem<BS>,
  __p: PhantomData<(P, Pcs)>,
}

impl<P, S, BS, Pcs> Default for Block<P, S, BS, Pcs>
  where S: Default,
        BS: Default
{
  fn default() -> Self {
    Block { inner: S::default(),
            block_states: Stem::new(BS::default()),
            __p: PhantomData }
  }
}

impl<P, S, BS, Pcs> Block<P, S, BS, Pcs> where P: PlatformTypes
{
  fn prune(&self, effects: &mut P::Effects, now: Instant<P::Clock>)
    where Pcs: Map<u32, Piece<Addrd<Message<P>>>>,
          BS: Array<Item = BlockState<P, Pcs>>
  {
    let len_before = self.block_states.map_ref(|bss| bss.len());
    fn go<_P, _S, _BS, _Pcs>(now: Instant<_P::Clock>, b: &Block<_P, _S, _BS, _Pcs>)
      where _P: PlatformTypes,
            _Pcs: Map<u32, Piece<Addrd<Message<_P>>>>,
            _BS: Array<Item = BlockState<_P, _Pcs>>
    {
      if let Some(ix) = b.block_states.map_ref(|bss| {
                                        bss.iter().enumerate().find_map(|(ix, b)| {
                                                                if now >= b.expires_at {
                                                                  Some(ix)
                                                                } else {
                                                                  None
                                                                }
                                                              })
                                      })
      {
        b.block_states.map_mut(|bss| bss.remove(ix));
        go(now, b);
      }
    }

    go(now, self);
    let len_after = self.block_states.map_ref(|bss| bss.len());
    if len_before - len_after > 0 {
      log!(Block::prune, effects, log::Level::Debug, "Removed {} expired entries. For outbound messages, a prior step SHOULD but MAY NOT retry sending them", len_before - len_after);
    }
  }

  fn find_rx_request_block_state_ix(&self, token: Addrd<Token>) -> Option<usize>
    where Pcs: Map<u32, Piece<Addrd<Message<P>>>>,
          BS: Array<Item = BlockState<P, Pcs>>
  {
    self.block_states.map_ref(|bs| {
                       bs.iter()
                         .enumerate()
                         .find(|(_, bs)| match bs.pcs.get(&0) {
                           | Some(Piece::Have(m)) => m.as_ref().map(|m| m.token) == token,
                           | _ => false,
                         })
                         .map(|(ix, _)| ix)
                     })
  }

  fn find_rx_response_block_state_ix(&self, token: Addrd<Token>, cache_key: u64) -> Option<usize>
    where Pcs: Map<u32, Piece<Addrd<Message<P>>>>,
          BS: Array<Item = BlockState<P, Pcs>>
  {
    self.block_states.map_ref(|block_states| {
                       block_states.iter()
                                   .enumerate()
                                   .find(|(_, bs)| {
                                     let block0_and_response_is_for_originating_request =
                                       bs.biggest.is_none()
                                       && bs.original
                                            .as_ref()
                                            .map(|msg| msg.as_ref().map(|m| m.token))
                                          == Some(token);

                                     let block_n_and_response_matches_previous_block =
                                       bs.pcs
                                         .get(&0)
                                         .and_then(|p| p.get_msg().map(|m| m.data().cache_key()))
                                       == Some(cache_key);

                                     block0_and_response_is_for_originating_request
                                     || block_n_and_response_matches_previous_block
                                   })
                                   .map(|(ix, _)| ix)
                     })
  }

  fn block_state_mut<F, R>(&self, ix: usize, f: F) -> R
    where F: for<'a> F1Once<&'a mut BlockState<P, Pcs>, Ret = R>,
          Pcs: Map<u32, Piece<Addrd<Message<P>>>>,
          BS: Array<Item = BlockState<P, Pcs>>
  {
    let mut f = Some(f);
    self.block_states
        .map_mut(|bs| Option::take(&mut f).unwrap().call1(&mut bs[ix]))
  }

  fn block_state<F, R>(&self, ix: usize, f: F) -> R
    where F: for<'a> F1Once<&'a BlockState<P, Pcs>, Ret = R>,
          Pcs: Map<u32, Piece<Addrd<Message<P>>>>,
          BS: Array<Item = BlockState<P, Pcs>>
  {
    let mut f = Some(f);
    self.block_states
        .map_ref(|bs| Option::take(&mut f).unwrap().call1(&bs[ix]))
  }

  fn push(&self, s: BlockState<P, Pcs>)
    where Pcs: Map<u32, Piece<Addrd<Message<P>>>>,
          BS: Array<Item = BlockState<P, Pcs>>
  {
    let mut s = Some(s);
    self.block_states
        .map_mut(|bs| bs.push(Option::take(&mut s).unwrap()));
  }

  fn remove(&self, ix: usize)
    where Pcs: Map<u32, Piece<Addrd<Message<P>>>>,
          BS: Array<Item = BlockState<P, Pcs>>
  {
    self.block_states.map_mut(|bs| bs.remove(ix));
  }

  fn clone_original(&self, ix: usize) -> Option<Addrd<Message<P>>>
    where Pcs: Map<u32, Piece<Addrd<Message<P>>>>,
          BS: Array<Item = BlockState<P, Pcs>>
  {
    self.block_state(ix, |s: &BlockState<_, _>| {
          s.original.as_ref().map(|orig| {
                               let addr = orig.addr();
                               let orig: &Message<P> = orig.data();
                               let mut new =
                                 Message::<P>::new(Type::Con, orig.code, Id(0), orig.token);

                               orig.opts.iter().for_each(|(n, vs)| {
                                                 if n.include_in_cache_key() {
                                                   new.opts.insert(*n, vs.clone()).ok();
                                                 }
                                               });

                               Addrd(new, addr)
                             })
        })
  }
}

impl<P, S, BS, Pcs> Step<P> for Block<P, S, BS, Pcs>
  where P: PlatformTypes,
        S: Step<P, PollReq = Addrd<Req<P>>, PollResp = Addrd<Resp<P>>>,
        Pcs: Map<u32, Piece<Addrd<Message<P>>>> + core::fmt::Debug,
        BS: Array<Item = BlockState<P, Pcs>>
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
        let rep_ty = if req.data().msg().ty == Type::Con {
          Type::Ack
        } else {
          Type::Non
        };

        let rep = Message::<P>::new(rep_ty, $code, Id(0), req.data().msg().token);
        effects.push(Effect::Send(Addrd(rep, req.addr())));
      }};
    }

    let block_state_ix = self.find_rx_request_block_state_ix(req.as_ref().map(|r| r.msg().token));

    match req.data().msg().block2() {
      | None => {
        if let Some(ix) = block_state_ix {
          self.block_state(ix, |s: &BlockState<_, _>| {
                log!(Block::poll_req,
                     effects,
                     log::Level::Warn,
                     "Expected message {:?} to continue block sequence {:?}",
                     req.data().msg().token,
                     s);
              });
          self.remove(ix);
          respond!(REQUEST_ENTITY_INCOMPLETE);
        }

        Some(Ok(req))
      },
      | Some(block) => {
        match block_state_ix {
          | None if block.num() > 0 => {
            respond!(REQUEST_ENTITY_INCOMPLETE);

            Some(Err(nb::Error::WouldBlock))
          },
          | None if block.more() => {
            let mut block_state =
              BlockState { biggest: Some(0),
                           original: None,
                           pcs: Default::default(),
                           expires_at: snap.time
                                       + Milliseconds(snap.config.exchange_lifetime_millis()) };

            block_state.have(snap.time, 0, req.clone().map(|r| r.into()));

            self.push(block_state);
            respond!(CONTINUE);

            Some(Err(nb::Error::WouldBlock))
          },
          | None => {
            // this is block 0 and there are no more blocks,
            // simply yield the request
            Some(Ok(req))
          },
          | Some(ix)
            if self.block_state(ix, BlockState::biggest)
                   .map(|n| n + 1)
                   .unwrap_or(0)
               < block.num() =>
          {
            self.remove(ix);
            respond!(REQUEST_ENTITY_INCOMPLETE);

            Some(Err(nb::Error::WouldBlock))
          },
          | Some(ix) => {
            self.block_state_mut(ix, |s: &mut BlockState<_, _>| {
                  s.have(snap.time, block.num(), req.clone().map(|r| r.into()))
                });

            if block.more() {
              respond!(CONTINUE);
              Some(Err(nb::Error::WouldBlock))
            } else {
              let p = self.block_state(ix, BlockState::assembled);
              req.as_mut().msg_mut().payload = p;
              self.remove(ix);
              Some(Ok(req))
            }
          },
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

    let block_state_ix = self.find_rx_response_block_state_ix(rep.as_ref().map(|r| r.msg().token),
                                                              rep.data().msg().cache_key());

    match rep.data().msg().block1() {
      | None => {
        // Response didn't have Block1; we can drop the block state
        if let Some(ix) = block_state_ix {
          self.remove(ix);
        }
        Some(Ok(rep))
      },
      | Some(block) => {
        match block_state_ix {
          | None => {
            // Got a Block1 message but we don't have any conception of it; yield the response as-is from the inner step.
            Some(Ok(rep))
          },
          | Some(ix) => {
            macro_rules! request_num {
              ($num:expr) => {{
                let mut new = self.clone_original(ix).unwrap();

                new.as_mut().set_block1(0, $num, false).ok();
                new.as_mut().remove(BLOCK2);
                new.as_mut().remove(SIZE2);

                effects.push(Effect::Send(new));
                self.block_state_mut(ix, |s: &mut BlockState<_, _>| s.waiting(snap.time, $num));
              }};
            }

            self.block_state_mut(ix, |s: &mut BlockState<_, _>| {
                  s.have(snap.time, block.num(), rep.clone().map(|r| r.into()))
                });

            if block.more() {
              request_num!(block.num() + 1);
            }

            if let Some(missing) = self.block_state(ix, BlockState::get_missing) {
              request_num!(missing);
            }

            if self.block_state(ix, BlockState::have_all) {
              rep.as_mut().msg_mut().payload = self.block_state(ix, BlockState::assembled);
              rep.as_mut().msg_mut().remove(BLOCK1);
              self.remove(ix);
              Some(Ok(rep))
            } else {
              Some(Err(nb::Error::WouldBlock))
            }
          },
        }
      },
    }
  }

  fn on_message_sent(&self,
                     snap: &platform::Snapshot<P>,
                     effs: &mut P::Effects,
                     msg: &Addrd<Message<P>>)
                     -> Result<(), Self::Error> {
    self.prune(effs, snap.time);
    self.inner.on_message_sent(snap, effs, msg)?;
    if msg.data().code.kind() == CodeKind::Request {
      self.push(BlockState { biggest: None,
                             original: Some(msg.clone()),
                             pcs: Default::default(),
                             expires_at: snap.time
                                         + Milliseconds(snap.config.exchange_lifetime_millis()) });
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

  #[test]
  fn ent_correctly_identifies_missing_pieces() {
    let mut e = BlockState::<test::Platform, BTreeMap<u32, Piece<()>>> { biggest: None,
                                                                         original: None,
                                                                         pcs: BTreeMap::new(),
                                                                         expires_at:
                                                                           Instant::new(1000) };
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
    let b = Block::<test::Platform, S, Vec<_>, BTreeMap<_, _>>::default();

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
    let b = Block::<test::Platform, S, Vec<_>, BTreeMap<_, _>>::default();

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
    let b = Block::<test::Platform, S, Vec<_>, BTreeMap<_, _>>::default();

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
  fn when_recv_request_with_block2_and_dont_hear_another_for_a_long_time_this_should_prune_state() {
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
    let b = Block::<test::Platform, S, Vec<_>, BTreeMap<_, _>>::default();

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

    assert!(matches!(b.poll_req(&t_0, &mut vec![]).unwrap().unwrap_err(), nb::Error::WouldBlock));
    assert_eq!(b.block_states.map_ref(|bss| bss.len()), 1);

    assert!(matches!(b.poll_req(&t_1, &mut vec![]), None));
    assert_eq!(b.block_states.map_ref(|bss| bss.len()), 1);

    assert!(matches!(b.poll_req(&t_2, &mut vec![]), None));
    assert_eq!(b.block_states.map_ref(|bss| bss.len()), 0);
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
    let b = Block::<test::Platform, S, Vec<_>, BTreeMap<_, _>>::default();

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

    assert_eq!(b.block_states.map_ref(|bss| bss.len()), 1);

    b.poll_resp(&t_n1, &mut effects, Token(Default::default()), addrs.server)
     .ok_or(())
     .unwrap_err();

    assert_eq!(b.block_states.map_ref(|bss| bss.len()), 1);

    b.poll_resp(&t_n2, &mut effects, Token(Default::default()), addrs.server)
     .ok_or(())
     .unwrap_err();

    assert_eq!(b.block_states.map_ref(|bss| bss.len()), 0);
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
    let b = Block::<test::Platform, S, Vec<_>, BTreeMap<_, _>>::default();

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
    let b = Block::<test::Platform, S, Vec<_>, BTreeMap<_, _>>::default();

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
    let b = Block::<test::Platform, S, Vec<_>, BTreeMap<_, _>>::default();

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
