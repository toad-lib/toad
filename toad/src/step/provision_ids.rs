use core::any::type_name;
use core::marker::PhantomData;

use embedded_time::duration::Milliseconds;
use embedded_time::Instant;
use no_std_net::SocketAddr;
use tinyvec::ArrayVec;
use toad_array::Array;
use toad_len::Len;
use toad_map::{InsertError, Map};
use toad_msg::Id;
use toad_stem::Stem;

use super::{Step, _try, log};
use crate::config::Config;
use crate::net::Addrd;
use crate::platform;
use crate::platform::PlatformTypes;
use crate::req::Req;
use crate::resp::Resp;
use crate::time::Stamped;

/// Supertrait type shenanigans
///
/// What we want: "given `A` which is an [`Array`] of `Item = `[`Id`],
/// I want a [`Map`] from [`SocketAddr`] to `A`."
///
/// This trait allows us to express that without adding noisy PhantomData
/// type parameters to the step, although it does add a minorly annoying restriction
/// that if you want to use something other than BTreeMap or ArrayVec,
/// you would have to wrap your collection in a newtype.
pub trait IdsBySocketAddr<P: PlatformTypes>: Map<SocketAddrWithDefault, Self::Ids> {
  /// the "given `A` which is an..." type above
  type Ids: Array<Item = Stamped<P::Clock, IdWithDefault>>;
}

#[cfg(feature = "alloc")]
impl<P: platform::PlatformTypes, A: Array<Item = Stamped<P::Clock, IdWithDefault>>>
  IdsBySocketAddr<P> for std_alloc::collections::BTreeMap<SocketAddrWithDefault, A>
{
  type Ids = A;
}

impl<P: platform::PlatformTypes,
      A: Array<Item = Stamped<P::Clock, IdWithDefault>>,
      const N: usize> IdsBySocketAddr<P> for ArrayVec<[(SocketAddrWithDefault, A); N]>
{
  type Ids = A;
}

/// Newtype wrapping [`no_std_net::SocketAddr`] that adds
/// a Default implementation.
///
/// Defined so that a [`tinyvec::ArrayVec`] may be used with this type.
///
/// This should be used sparingly, since a "default socket address"
/// isn't meaningful
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
#[non_exhaustive]
pub struct SocketAddrWithDefault(pub SocketAddr);

impl Default for SocketAddrWithDefault {
  fn default() -> Self {
    use no_std_net::*;
    Self(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 0)))
  }
}

/// Newtype wrapping [`toad_msg::Id`] that adds a Default implementation.
///
/// Defined so that a [`tinyvec::ArrayVec`] may be used with this type.
///
/// This should be used sparingly, since a "default message id"
/// isn't meaningful
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub struct IdWithDefault(pub Id);

impl Default for IdWithDefault {
  fn default() -> Self {
    Self(Id(0))
  }
}

/// Step responsible for setting the token of all outbound messages with
/// empty ids (`Id(0)`, assumed to be meaningless)
/// with a new id that is guaranteed to be unique to the conversation with
/// the message's origin/destination address.
///
/// For more information, see the [module documentation](crate::step::provision_ids).
#[derive(Debug)]
pub struct ProvisionIds<P, Inner, SeenIds> {
  inner: Inner,
  seen: Stem<SeenIds>,
  __p: PhantomData<P>,
}

impl<P, Inner, SeenIds> Default for ProvisionIds<P, Inner, SeenIds>
  where Inner: Default,
        SeenIds: Default
{
  fn default() -> Self {
    Self { inner: Default::default(),
           seen: Default::default(),
           __p: PhantomData }
  }
}

impl<P, Inner, Ids> ProvisionIds<P, Inner, Ids>
  where Ids: IdsBySocketAddr<P>,
        P: PlatformTypes
{
  fn prune(effs: &mut P::Effects, seen: &mut Ids, now: Instant<P::Clock>, config: Config) {
    for (_, ids) in seen.iter_mut() {
      ids.sort_by_key(|t| t.time());
      let ix_of_first_id_to_keep = ids.iter()
                                      .enumerate()
                                      .find(|(_, id)| {
                                        now.checked_duration_since(&id.time())
                               < Some(Milliseconds(config.exchange_lifetime_millis()).into())
                                      })
                                      .map(|(ix, _)| ix);

      match ix_of_first_id_to_keep {
        | Some(keep_at) if keep_at == 0 => (),
        | Some(keep_at) => {
          log!(ProvisionIds::prune,
               effs,
               log::Level::Trace,
               "removing {} old irrelevant ids",
               keep_at);
          for ix in 0..keep_at {
            ids.remove(ix);
          }
        },
        | None => {
          // there is no index of id that should be kept
          *ids = Default::default();
        },
      }
    }
  }

  fn new_addr(effs: &mut P::Effects, seen: &mut Ids, addr: SocketAddr) {
    log!(ProvisionIds::new_addr,
         effs,
         log::Level::Trace,
         "haven't seen {:?} before",
         addr);
    match seen.insert(SocketAddrWithDefault(addr), Default::default()) {
      | Ok(_) => (),
      | Err(InsertError::CapacityExhausted) => {
        let mut to_remove: Option<Stamped<P::Clock, SocketAddrWithDefault>> = None;

        for (addr, ids) in seen.iter_mut() {
          if ids.is_empty() {
            to_remove = Some(Stamped(*addr, Instant::new(0)));
            break;
          }

          ids.sort_by_key(|t| t.time());
          let newest_id_time = ids[ids.len() - 1].time();
          ids.sort();

          // is the newest id for this addr older than the newest id for `to_remove`?
          if to_remove.is_none() || Some(newest_id_time) < to_remove.map(|t| t.time()) {
            to_remove = Some(Stamped(*addr, newest_id_time));
          }
        }

        seen.remove(&to_remove.unwrap().discard_timestamp());
      },
      | Err(InsertError::Exists(_)) => unreachable!(),
    };
  }

  /// Generate a Message ID that has not been used yet with the connection with this socket
  ///
  /// best case O(1), worst case O(n)
  fn next(effs: &mut P::Effects,
          seen: &mut Ids,
          config: Config,
          time: Instant<P::Clock>,
          addr: SocketAddr)
          -> Id {
    match seen.get_mut(&SocketAddrWithDefault(addr)) {
      | None => {
        Self::new_addr(effs, seen, addr);
        Self::next(effs, seen, config, time, addr)
      },
      | Some(ids) => {
        // Pessimistically assume clients are sending us non-sequential
        // IDs and sort every time we need a new one.
        //
        // Because we're sorting often, then it will always be
        // /almost/ sorted after insert, so this should have
        // a negligible perf penalty.
        ids.sort_unstable();

        let smallest = || ids[0].data().0 .0;
        let biggest = || ids[ids.len() - 1].data().0 .0;

        let next = if ids.is_empty() {
          Id(1)
        } else if biggest() < u16::MAX {
          Id(biggest() + 1)
        } else if smallest() > 1 {
          Id(smallest() - 1)
        } else {
          let mut ahead = ids.iter();
          ahead.next();

          // # PANICS
          //
          // ideally `ids` will always be a sequence of natural numbers
          // (1, 2, 3, 4, ..)
          //
          // if the smallest is 1, and the biggest is u16::MAX, then our only hope
          // is that `ids` skips some natural numbers.
          //
          // e.g. 3 is skipped in (1, 2, 4, 5, ..)
          //
          // if this is the case, we can get a unique ID by finding a gap (2, 4)
          // and adding 1 to the integer at the start of the gap.
          //
          // if the set of ids is literally **EVERY** integer in u16 then this will panic.
          let (Stamped(IdWithDefault(Id(before_gap)), _), _) =
            ids.iter()
               .zip(ahead)
               .find(|(Stamped(IdWithDefault(Id(cur)), _), Stamped(IdWithDefault(Id(next)), _))| {
                       next - cur > 1
                     })
               .unwrap();
          Id(before_gap + 1)
        };

        log!(ProvisionIds::next,
             effs,
             log::Level::Debug,
             "Generated new {:?}",
             next);
        Self::seen(effs, seen, config, time, addr, next);
        next
      },
    }
  }

  /// Mark an Id + Addr pair as being seen at `time`.
  fn seen(effs: &mut P::Effects,
          seen: &mut Ids,
          config: Config,
          now: Instant<P::Clock>,
          addr: SocketAddr,
          id: Id) {
    Self::prune(effs, seen, now, config);

    match seen.get_mut(&SocketAddrWithDefault(addr)) {
      | None => {
        Self::new_addr(effs, seen, addr);
        Self::seen(effs, seen, config, now, addr, id)
      },
      | Some(ids) => {
        if ids.is_full() {
          // Incorrect but unavoidable:
          //
          // On the small chance that we've reached the capacity for the Id buffer,
          // we have 3 options:
          //  * error
          //  * panic
          //  * make room for the new one
          //
          // Here we choose to remove the oldest Id so that we can make room for the newest.
          //
          // Assuming that:
          //  * the oldest should be the smallest
          //  * there aren't excessive gaps in the Ids seen
          //
          // Then it won't be reused until we hit u16::MAX, then overflow and count up to it.
          //
          // By this time hopefully the max_transmit_span (how long Ids are expected to remain unique)
          // will have passed.
          log!(ProvisionIds::seen, effs, log::Level::Warn, "Id buffer {} has reached capacity of {}. Forgetting the oldest Id to make room for {:?}", type_name::<Ids>(), Ids::CAPACITY.unwrap_or(usize::MAX), id);

          ids.sort_by_key(|s| s.time());
          ids.remove(0);
          ids.sort();
        }

        log!(ProvisionIds::seen,
             effs,
             log::Level::Trace,
             "Saw new {:?}",
             id);
        ids.push(Stamped(IdWithDefault(id), now));
      },
    }
  }
}

macro_rules! common {
  ($self:expr, $effs:expr, $snap:expr, $req_or_resp:expr) => {{
    let r = $req_or_resp;
    $self.seen.map_mut(|s| {
                Self::seen($effs,
                           s,
                           $snap.config,
                           $snap.time,
                           r.addr(),
                           r.data().msg().id)
              });
    Some(Ok(r))
  }};
}

impl<P, E: super::Error, Inner, Ids> Step<P> for ProvisionIds<P, Inner, Ids>
  where P: PlatformTypes,
        Inner: Step<P, PollReq = Addrd<Req<P>>, PollResp = Addrd<Resp<P>>, Error = E>,
        Ids: IdsBySocketAddr<P>
{
  type PollReq = Addrd<Req<P>>;
  type PollResp = Addrd<Resp<P>>;
  type Error = E;
  type Inner = Inner;

  fn inner(&self) -> &Inner {
    &self.inner
  }

  fn poll_req(&self,
              snap: &crate::platform::Snapshot<P>,
              effects: &mut <P as PlatformTypes>::Effects)
              -> super::StepOutput<Self::PollReq, Self::Error> {
    let req = self.inner.poll_req(snap, effects);
    let req = _try!(Option<nb::Result>; req);
    common!(self, effects, snap, req)
  }

  fn poll_resp(&self,
               snap: &crate::platform::Snapshot<P>,
               effects: &mut <P as PlatformTypes>::Effects,
               token: toad_msg::Token,
               addr: SocketAddr)
               -> super::StepOutput<Self::PollResp, Self::Error> {
    let resp = self.inner.poll_resp(snap, effects, token, addr);
    let resp = _try!(Option<nb::Result>; resp);
    common!(self, effects, snap, resp)
  }

  fn before_message_sent(&self,
                         snap: &platform::Snapshot<P>,
                         effs: &mut P::Effects,
                         msg: &mut Addrd<platform::Message<P>>)
                         -> Result<(), Self::Error> {
    self.inner.before_message_sent(snap, effs, msg)?;

    if msg.data().id == Id(0) {
      let id = self.seen
                   .map_mut(|s| Self::next(effs, s, snap.config, snap.time, msg.addr()));
      msg.data_mut().id = id;
    }

    Ok(())
  }
}

#[cfg(test)]
mod test {
  use std::collections::BTreeMap;

  use embedded_time::duration::Microseconds;

  use super::*;
  use crate::step::test::test_step;
  use crate::test::{self, ClockMock, Platform as P};

  type InnerPollReq = Addrd<Req<test::Platform>>;
  type InnerPollResp = Addrd<Resp<test::Platform>>;
  type ProvisionIds<S> = super::ProvisionIds<P,
                                             S,
                                             BTreeMap<SocketAddrWithDefault,
                                                      Vec<Stamped<ClockMock, IdWithDefault>>>>;

  fn test_msg(id: Id) -> Addrd<test::Message> {
    use toad_msg::*;

    Addrd(test::Message { id,
                          ty: Type::Con,
                          ver: Default::default(),
                          code: Code::new(0, 0),
                          opts: Default::default(),
                          payload: Payload(vec![]),
                          token: Token(Default::default()) },
          test::dummy_addr())
  }

  test_step!(
    GIVEN ProvisionIds::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
    WHEN inner_errors [
      (inner.poll_req => { Some(Err(nb::Error::Other(()))) }),
      (inner.poll_resp => { Some(Err(nb::Error::Other(()))) })
    ]
    THEN this_should_error [
      (poll_req(_, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::Other(())))) }),
      (poll_resp(_, _, _, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::Other(())))) })
    ]
  );

  test_step!(
    GIVEN ProvisionIds::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
    WHEN inner_blocks [
      (inner.poll_req => { Some(Err(nb::Error::WouldBlock)) }),
      (inner.poll_resp => { Some(Err(nb::Error::WouldBlock)) })
    ]
    THEN this_should_block [
      (poll_req(_, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::WouldBlock))) }),
      (poll_resp(_, _, _, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::WouldBlock))) })
    ]
  );

  test_step!(
    GIVEN ProvisionIds::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
    WHEN message_sent_with_id_zero []
    THEN this_should_assign_nonzero_id [
      (before_message_sent(_, _, test_msg(Id(0))) should be ok with { |msg| assert!(matches!(msg.data().id, Id(n) if n > 0)) })
    ]
  );

  test_step!(
    GIVEN ProvisionIds::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
    WHEN req_or_resp_recvd_with_id_zero [
      (inner.poll_req => { Some(Ok(test_msg(Id(0)).map(Req::from))) }),
      (inner.poll_resp => { Some(Ok(test_msg(Id(0)).map(Resp::from))) })
    ]
    THEN id_should_be_respected [
      (poll_req(_, _) should satisfy { |out| assert!(matches!(out.unwrap().unwrap().data().as_ref().id, Id(0))) }),
      (poll_resp(_, _, _, _) should satisfy { |out| assert!(matches!(out.unwrap().unwrap().data().as_ref().id, Id(0))) })
    ]
  );

  #[test]
  fn seen_should_remove_oldest_addr_when_new_addr_would_exceed_capacity() {
    type Ids = ArrayVec<[Stamped<ClockMock, IdWithDefault>; 16]>;
    type IdsByAddr = ArrayVec<[(SocketAddrWithDefault, Ids); 2]>;
    type Step = super::ProvisionIds<P, (), IdsByAddr>;

    let mut effs = Vec::<test::Effect>::new();
    let step = Step::default();
    let cfg = Config::default();

    step.seen.map_mut(|s| {
               Step::seen(&mut effs,
                          s,
                          cfg,
                          ClockMock::instant(0),
                          test::dummy_addr(),
                          Id(1));
               Step::seen(&mut effs,
                          s,
                          cfg,
                          ClockMock::instant(1),
                          test::dummy_addr_2(),
                          Id(1));
               Step::seen(&mut effs,
                          s,
                          cfg,
                          ClockMock::instant(2),
                          test::dummy_addr(),
                          Id(2));
               Step::seen(&mut effs,
                          s,
                          cfg,
                          ClockMock::instant(3),
                          test::dummy_addr_3(),
                          Id(1));
             });

    let mut addrs: Vec<_> = step.seen.map_ref(|s| s.iter().map(|(k, _)| k.0).collect());
    addrs.sort();

    assert_eq!(addrs, vec![test::dummy_addr(), test::dummy_addr_3()]);
  }

  #[test]
  fn seen_should_remove_empty_addr_when_new_addr_would_exceed_capacity() {
    type Ids = ArrayVec<[Stamped<ClockMock, IdWithDefault>; 16]>;
    type IdsByAddr = ArrayVec<[(SocketAddrWithDefault, Ids); 2]>;
    type Step = super::ProvisionIds<P, (), IdsByAddr>;

    let mut effs = Vec::<test::Effect>::new();
    let step = Step::default();
    let cfg = Config::default();

    step.seen.map_mut(|seen| {
               Map::insert(seen,
                           SocketAddrWithDefault(test::dummy_addr()),
                           Default::default()).unwrap();
               Step::seen(&mut effs,
                          seen,
                          cfg,
                          ClockMock::instant(1),
                          test::dummy_addr_2(),
                          Id(1));
               Step::seen(&mut effs,
                          seen,
                          cfg,
                          ClockMock::instant(3),
                          test::dummy_addr_3(),
                          Id(1));
             });

    let mut addrs: Vec<_> = step.seen.map_ref(|s| s.iter().map(|(k, _)| k.0).collect());
    addrs.sort();

    assert_eq!(addrs, vec![test::dummy_addr_2(), test::dummy_addr_3()]);
  }

  #[test]
  fn seen_should_remove_oldest_id_when_about_to_exceed_capacity() {
    type Ids = ArrayVec<[Stamped<ClockMock, IdWithDefault>; 2]>;
    type IdsByAddr = ArrayVec<[(SocketAddrWithDefault, Ids); 1]>;
    type Step = super::ProvisionIds<P, (), IdsByAddr>;

    let mut effs = Vec::<test::Effect>::new();
    let step = Step::default();
    let cfg = Config::default();

    step.seen.map_mut(|seen| {
               Step::seen(&mut effs,
                          seen,
                          cfg,
                          ClockMock::instant(0),
                          test::dummy_addr(),
                          Id(0));
               Step::seen(&mut effs,
                          seen,
                          cfg,
                          ClockMock::instant(1),
                          test::dummy_addr(),
                          Id(1));
               Step::seen(&mut effs,
                          seen,
                          cfg,
                          ClockMock::instant(2),
                          test::dummy_addr(),
                          Id(2));
             });

    let ids: Vec<_> = step.seen.map_ref(|s| {
                                 s.get(&SocketAddrWithDefault(test::dummy_addr()))
                                  .unwrap()
                                  .into_iter()
                                  .map(|Stamped(IdWithDefault(id), _)| *id)
                                  .collect()
                               });
    assert_eq!(ids, vec![Id(1), Id(2)]);
  }

  #[test]
  fn seen_should_prune_ids_older_than_exchange_lifetime() {
    type Step = ProvisionIds<()>;

    let mut effs = Vec::<test::Effect>::new();
    let step = Step::default();
    let cfg = Config::default();
    let exchange_lifetime_micros = cfg.exchange_lifetime_millis() * 1_000;

    // This test assumes that the clock considers 1 "tick" to be 1 microsecond.
    assert_eq!(Microseconds::try_from(ClockMock::instant(1).duration_since_epoch()),
               Ok(Microseconds(1u64)));

    step.seen.map_mut(|s| {
               Step::seen(&mut effs,
                          s,
                          cfg,
                          ClockMock::instant(0),
                          test::dummy_addr(),
                          Id(1));
               Step::seen(&mut effs,
                          s,
                          cfg,
                          ClockMock::instant(1),
                          test::dummy_addr(),
                          Id(2));
               Step::seen(&mut effs,
                          s,
                          cfg,
                          ClockMock::instant(exchange_lifetime_micros + 1_000),
                          test::dummy_addr(),
                          Id(3));
             });

    let ids: Vec<_> = step.seen.map_ref(|s| {
                                 s.get(&SocketAddrWithDefault(test::dummy_addr()))
                                  .unwrap()
                                  .iter()
                                  .map(|Stamped(IdWithDefault(id), _)| *id)
                                  .collect()
                               });
    assert_eq!(ids, vec![Id(3)]);
  }

  #[test]
  fn next_should_generate_largest_plus_one_when_largest_lt_max() {
    type Step = ProvisionIds<()>;

    let mut effs = Vec::<test::Effect>::new();
    let step = Step::default();
    let time = ClockMock::instant(0);

    step.seen.map_mut(|seen| {
               Step::seen(&mut effs,
                          seen,
                          Default::default(),
                          time,
                          test::dummy_addr(),
                          Id(22));
               Step::seen(&mut effs,
                          seen,
                          Default::default(),
                          time,
                          test::dummy_addr(),
                          Id(1));
               Step::seen(&mut effs,
                          seen,
                          Default::default(),
                          time,
                          test::dummy_addr(),
                          Id(2));

               let generated = Step::next(&mut effs,
                                          seen,
                                          Default::default(),
                                          time,
                                          test::dummy_addr());
               assert_eq!(generated, Id(23))
             });
  }

  #[test]
  fn next_should_generate_smallest_minus_one_when_largest_is_max() {
    type Step = ProvisionIds<()>;

    let mut effs = Vec::<test::Effect>::new();
    let step = Step::default();
    let time = ClockMock::instant(0);

    step.seen.map_mut(|seen| {
               Step::seen(&mut effs,
                          seen,
                          Default::default(),
                          time,
                          test::dummy_addr(),
                          Id(2));
               Step::seen(&mut effs,
                          seen,
                          Default::default(),
                          time,
                          test::dummy_addr(),
                          Id(u16::MAX));

               let generated = Step::next(&mut effs,
                                          seen,
                                          Default::default(),
                                          time,
                                          test::dummy_addr());
               assert_eq!(generated, Id(1))
             });
  }

  #[test]
  fn next_should_generate_in_gap_when_smallest_1_and_largest_max() {
    type Step = ProvisionIds<()>;

    let mut effs = Vec::<test::Effect>::new();
    let step = Step::default();
    let time = ClockMock::instant(0);

    step.seen.map_mut(|seen| {
               Step::seen(&mut effs,
                          seen,
                          Default::default(),
                          time,
                          test::dummy_addr(),
                          Id(1));
               Step::seen(&mut effs,
                          seen,
                          Default::default(),
                          time,
                          test::dummy_addr(),
                          Id(2));
               Step::seen(&mut effs,
                          seen,
                          Default::default(),
                          time,
                          test::dummy_addr(),
                          Id(3));
               Step::seen(&mut effs,
                          seen,
                          Default::default(),
                          time,
                          test::dummy_addr(),
                          Id(5));
               Step::seen(&mut effs,
                          seen,
                          Default::default(),
                          time,
                          test::dummy_addr(),
                          Id(u16::MAX));

               let generated = Step::next(&mut effs,
                                          seen,
                                          Default::default(),
                                          time,
                                          test::dummy_addr());
               assert_eq!(generated, Id(4))
             });
  }

  #[test]
  fn next_should_generate_initial_id() {
    type Step = ProvisionIds<()>;
    let step = Step::default();
    let id = step.seen.map_mut(|s| {
                        Step::next(&mut vec![],
                                   s,
                                   Default::default(),
                                   ClockMock::instant(0),
                                   test::dummy_addr())
                      });
    assert_eq!(id, Id(1))
  }
}
