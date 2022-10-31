use core::marker::PhantomData;

use embedded_time::duration::Milliseconds;
use embedded_time::Instant;
use no_std_net::SocketAddr;
use tinyvec::ArrayVec;
use toad_common::{Array, GetSize, InsertError, Map};
use toad_msg::Id;

use super::{Step, _try};
use crate::config::ConfigData;
use crate::net::Addrd;
use crate::platform;
use crate::platform::Platform;
use crate::req::Req;
use crate::resp::Resp;
use crate::time::Stamped;

/// `ProvisionIds` that uses BTreeMap
///
/// Only enabled when feature "alloc" enabled.
#[cfg(feature = "alloc")]
pub mod alloc {
  use ::std_alloc::collections::BTreeMap;

  use super::*;

  type AllIds<P> = Vec<Stamped<<P as Platform>::Clock, IdWithDefault>>;

  type Map<P> = BTreeMap<SocketAddrWithDefault, AllIds<P>>;

  /// `ProvisionIds` that uses BTreeMap
  ///
  /// Only enabled when feature "alloc" enabled.
  ///
  /// For more information see [`super::ProvisionIds`]
  /// or the [module documentation](crate::step::provision_ids).
  pub type ProvisionIds<P, S> = super::ProvisionIds<P, S, Map<P>>;
}

/// `ProvisionIds` that uses ArrayVec, storing Ids on
/// the stack.
pub mod no_alloc {
  use super::*;
  use crate::todo::StackMap;

  type AllIds<P, const ID_BUFFER_SIZE: usize> =
    ArrayVec<[Stamped<<P as Platform>::Clock, IdWithDefault>; ID_BUFFER_SIZE]>;

  type Map<P, const ID_BUFFER_SIZE: usize, const MAX_ADDRS: usize> =
    StackMap<SocketAddrWithDefault, AllIds<P, ID_BUFFER_SIZE>, MAX_ADDRS>;

  /// `ProvisionIds` that uses ArrayVec, storing Ids on
  /// the stack.
  ///
  /// For more information see [`super::ProvisionIds`]
  /// or the [module documentation](crate::step::provision_ids).
  pub type ProvisionIds<P, S, const ID_BUFFER_SIZE: usize, const MAX_ADDRS: usize> =
    super::ProvisionIds<P, S, Map<P, ID_BUFFER_SIZE, MAX_ADDRS>>;
}

/// Supertrait type shenanigans
///
/// What we want: "given `A` which is an [`Array`] of `Item = `[`Id`],
/// I want a [`Map`] from [`SocketAddr`] to `A`."
///
/// This trait allows us to express that without adding noisy PhantomData
/// type parameters to the step, although it does add a minorly annoying restriction
/// that if you want to use something other than BTreeMap or ArrayVec,
/// you would have to wrap your collection in a newtype.
pub trait IdsBySocketAddr<P: Platform>: Map<SocketAddrWithDefault, Self::Ids> {
  /// the "given `A` which is an..." type above
  type Ids: Array<Item = Stamped<P::Clock, IdWithDefault>>;
}

#[cfg(feature = "alloc")]
impl<P: platform::Platform, A: Array<Item = Stamped<P::Clock, IdWithDefault>>> IdsBySocketAddr<P>
  for std_alloc::collections::BTreeMap<SocketAddrWithDefault, A>
{
  type Ids = A;
}

impl<P: platform::Platform, A: Array<Item = Stamped<P::Clock, IdWithDefault>>, const N: usize>
  IdsBySocketAddr<P> for ArrayVec<[(SocketAddrWithDefault, A); N]>
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

/// Step responsible for replacing all message ids of zero `Id(0)` (assumed to be meaningless)
/// with a new meaningful Id that is guaranteed to be unique to the conversation with
/// the message's origin/destination address.
#[derive(Debug, Clone)]
pub struct ProvisionIds<P, Inner, SeenIds> {
  inner: Inner,
  seen: SeenIds,
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
        P: Platform
{
  fn prune(&mut self, now: Instant<P::Clock>, config: ConfigData) {
    for (_, ids) in self.seen.iter_mut() {
      ids.sort_by_key(|t| t.time());
      let remove_before =
        ids.iter()
           .enumerate()
           .find(|(_, id)| now - id.time() < Milliseconds(config.exchange_lifetime_millis()).into())
           .map(|(ix, _)| ix);

      match remove_before {
        | Some(ix) if ix == 0 => (),
        | Some(ix) => {
          for ix in 0..ix {
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

  fn new_addr(&mut self, addr: SocketAddr) {
    match self.seen
              .insert(SocketAddrWithDefault(addr), Default::default())
    {
      | Ok(_) => (),
      | Err(InsertError::CapacityExhausted) => {
        let mut to_remove: Option<Stamped<P::Clock, SocketAddrWithDefault>> = None;

        for (addr, ids) in self.seen.iter_mut() {
          if ids.is_empty() {
            to_remove = Some(Stamped(*addr, Instant::new(0)));
            break;
          }

          ids.sort_by_key(|t| t.time());
          let newest_id_time = ids[ids.get_size() - 1].time();
          ids.sort();

          // is the newest id for this addr older than the newest id for `to_remove`?
          if to_remove.is_none() || Some(newest_id_time) < to_remove.map(|t| t.time()) {
            to_remove = Some(Stamped(*addr, newest_id_time));
          }
        }

        self.seen.remove(&to_remove.unwrap().discard_timestamp());
      },
      | Err(InsertError::Exists(_)) => unreachable!(),
    };
  }

  /// Generate a Message ID that has not been used yet with the connection with this socket
  ///
  /// best case O(1), worst case O(n)
  fn next(&mut self, config: ConfigData, time: Instant<P::Clock>, addr: SocketAddr) -> Id {
    match self.seen.get_mut(&SocketAddrWithDefault(addr)) {
      | None => {
        self.new_addr(addr);
        self.next(config, time, addr)
      },
      | Some(ids) => {
        // Pessimistically assume clients are sending us non-sequential
        // IDs and sort every time we need a new one.
        //
        // Because we should be sorting frequently, this should have
        // a negligible perf penalty.
        ids.sort_unstable();

        let smallest = || ids[0].data().0 .0;
        let biggest = || ids[ids.get_size() - 1].data().0 .0;

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

        self.seen(config, time, addr, next);
        next
      },
    }
  }

  /// Mark an Id + Addr pair as being seen at `time`.
  fn seen(&mut self, config: ConfigData, now: Instant<P::Clock>, addr: SocketAddr, id: Id) {
    self.prune(now, config);

    match self.seen.get_mut(&SocketAddrWithDefault(addr)) {
      | None => {
        self.new_addr(addr);
        self.seen(config, now, addr, id)
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
          ids.sort_by_key(|s| s.time());
          ids.remove(0);
          ids.sort();
        }

        ids.push(Stamped(IdWithDefault(id), now));
      },
    }
  }
}

macro_rules! common {
  ($self:expr, $snap:expr, $req_or_resp:expr) => {{
    let mut r = $req_or_resp;
    let addr = r.addr();
    let id = &mut r.data_mut().msg.id;

    if *id == Id(0) {
      let new = $self.next($snap.config, $snap.time, addr);
      *id = new;
    } else {
      $self.seen($snap.config, $snap.time, addr, *id);
    }

    Some(Ok(r))
  }};
}

impl<P, E: super::Error, Inner, Ids> Step<P> for ProvisionIds<P, Inner, Ids>
  where P: Platform,
        Inner: Step<P, PollReq = Addrd<Req<P>>, PollResp = Addrd<Resp<P>>, Error = E>,
        Ids: IdsBySocketAddr<P>
{
  type PollReq = Addrd<Req<P>>;
  type PollResp = Addrd<Resp<P>>;
  type Error = E;
  type Inner = Inner;

  fn inner(&mut self) -> &mut Self::Inner {
    &mut self.inner
  }

  fn poll_req(&mut self,
              snap: &crate::platform::Snapshot<P>,
              effects: &mut <P as Platform>::Effects)
              -> super::StepOutput<Self::PollReq, Self::Error> {
    let req = self.inner.poll_req(snap, effects);
    let req = _try!(Option<nb::Result>; req);
    common!(self, snap, req)
  }

  fn poll_resp(&mut self,
               snap: &crate::platform::Snapshot<P>,
               effects: &mut <P as Platform>::Effects,
               token: toad_msg::Token,
               addr: SocketAddr)
               -> super::StepOutput<Self::PollResp, Self::Error> {
    let resp = self.inner.poll_resp(snap, effects, token, addr);
    let resp = _try!(Option<nb::Result>; resp);
    common!(self, snap, resp)
  }

  fn before_message_sent(&mut self,
                         snap: &platform::Snapshot<P>,
                         msg: &mut Addrd<platform::Message<P>>)
                         -> Result<(), Self::Error> {
    self.inner.before_message_sent(snap, msg)?;

    if msg.data().id == Id(0) {
      let id = self.next(snap.config, snap.time, msg.addr());
      msg.data_mut().id = id;
    }

    Ok(())
  }
}

#[cfg(test)]
mod test {
  use toad_common::Map;

  use super::*;
  use crate::step::test::test_step;
  use crate::test::{ClockMock, Platform as P};

  type InnerPollReq = Addrd<Req<crate::test::Platform>>;
  type InnerPollResp = Addrd<Resp<crate::test::Platform>>;

  fn test_msg(id: Id) -> Addrd<crate::test::Message> {
    use toad_msg::*;

    Addrd(crate::test::Message { id,
                                 ty: Type::Con,
                                 ver: Default::default(),
                                 code: Code::new(0, 0),
                                 opts: vec![],
                                 payload: Payload(vec![]),
                                 token: Token(Default::default()) },
          crate::test::dummy_addr())
  }

  test_step!(
    GIVEN alloc::ProvisionIds::<P, Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
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
    GIVEN alloc::ProvisionIds::<P, Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
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
    GIVEN alloc::ProvisionIds::<P, Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
    WHEN inner_yields_id_zero [
      (inner.poll_req => { Some(Ok(test_msg(Id(0)).map(Req::from))) }),
      (inner.poll_resp => { Some(Ok(test_msg(Id(0)).map(Resp::from))) })
    ]
    THEN this_should_assign_nonzero_id [
      (poll_req(_, _) should satisfy { |out| assert!(matches!(out.unwrap().unwrap().data().msg.id, Id(n) if n > 0)) }),
      (poll_resp(_, _, _, _) should satisfy { |out| assert!(matches!(out.unwrap().unwrap().data().msg.id, Id(n) if n > 0)) }),
      (before_message_sent(_, test_msg(Id(0))) should be ok with { |msg| assert!(matches!(msg.data().id, Id(n) if n > 0)) })
    ]
  );

  #[test]
  fn seen_should_remove_oldest_addr_when_new_addr_would_exceed_capacity() {
    let mut step = no_alloc::ProvisionIds::<P, (), 16, 2>::default();
    let cfg = ConfigData::default();

    step.seen(cfg, ClockMock::instant(0), crate::test::dummy_addr(), Id(1));
    step.seen(cfg,
              ClockMock::instant(1),
              crate::test::dummy_addr_2(),
              Id(1));
    step.seen(cfg, ClockMock::instant(2), crate::test::dummy_addr(), Id(2));
    step.seen(cfg,
              ClockMock::instant(3),
              crate::test::dummy_addr_3(),
              Id(1));

    let mut addrs: Vec<_> = step.seen.iter().map(|(k, _)| k.0).collect();
    addrs.sort();

    assert_eq!(addrs,
               vec![crate::test::dummy_addr(), crate::test::dummy_addr_3()]);
  }

  #[test]
  fn seen_should_remove_empty_addr_when_new_addr_would_exceed_capacity() {
    let mut step = no_alloc::ProvisionIds::<P, (), 16, 2>::default();
    let cfg = ConfigData::default();

    Map::insert(&mut step.seen,
                SocketAddrWithDefault(crate::test::dummy_addr()),
                Default::default()).unwrap();
    step.seen(cfg,
              ClockMock::instant(1),
              crate::test::dummy_addr_2(),
              Id(1));
    step.seen(cfg,
              ClockMock::instant(3),
              crate::test::dummy_addr_3(),
              Id(1));

    let mut addrs: Vec<_> = step.seen.iter().map(|(k, _)| k.0).collect();
    addrs.sort();

    assert_eq!(addrs,
               vec![crate::test::dummy_addr_2(), crate::test::dummy_addr_3()]);
  }

  #[test]
  fn seen_should_remove_oldest_id_when_about_to_exceed_capacity() {
    let mut step = no_alloc::ProvisionIds::<P, (), 2, 1>::default();
    let cfg = ConfigData::default();

    step.seen(cfg, ClockMock::instant(0), crate::test::dummy_addr(), Id(0));
    step.seen(cfg, ClockMock::instant(1), crate::test::dummy_addr(), Id(1));
    step.seen(cfg, ClockMock::instant(2), crate::test::dummy_addr(), Id(2));

    let ids: Vec<_> = step.seen
                          .get(&SocketAddrWithDefault(crate::test::dummy_addr()))
                          .unwrap()
                          .into_iter()
                          .map(|Stamped(IdWithDefault(id), _)| id)
                          .collect();
    assert_eq!(ids, vec![&Id(1), &Id(2)]);
  }

  #[test]
  fn seen_should_prune_ids_older_than_exchange_lifetime() {
    let mut step = alloc::ProvisionIds::<P, ()>::default();
    let cfg = ConfigData::default();

    // let's make sure that the exchange lifetime is what we expect,
    // and that the clock considers 1 "tick" to be a nanosecond
    assert_eq!(cfg.exchange_lifetime_millis(), 212_200);
    assert_eq!(Milliseconds::try_from(ClockMock::instant(212_200_000).duration_since_epoch()),
               Ok(Milliseconds(212_200u64)));

    step.seen(cfg, ClockMock::instant(0), crate::test::dummy_addr(), Id(1));
    step.seen(cfg, ClockMock::instant(1), crate::test::dummy_addr(), Id(2));
    step.seen(cfg,
              ClockMock::instant(212_201_000),
              crate::test::dummy_addr(),
              Id(3));

    let ids: Vec<_> = step.seen
                          .get(&SocketAddrWithDefault(crate::test::dummy_addr()))
                          .unwrap()
                          .into_iter()
                          .map(|Stamped(IdWithDefault(id), _)| id)
                          .collect();
    assert_eq!(ids, vec![&Id(3)]);
  }

  #[test]
  fn next_should_generate_largest_plus_one_when_largest_lt_max() {
    let mut step = alloc::ProvisionIds::<P, ()>::default();
    let time = ClockMock::instant(0);

    step.seen(Default::default(), time, crate::test::dummy_addr(), Id(22));
    step.seen(Default::default(), time, crate::test::dummy_addr(), Id(1));
    step.seen(Default::default(), time, crate::test::dummy_addr(), Id(2));

    let generated = step.next(Default::default(), time, crate::test::dummy_addr());
    assert_eq!(generated, Id(23))
  }

  #[test]
  fn next_should_generate_smallest_minus_one_when_largest_is_max() {
    let mut step = alloc::ProvisionIds::<P, ()>::default();
    let time = ClockMock::instant(0);

    step.seen(Default::default(), time, crate::test::dummy_addr(), Id(2));
    step.seen(Default::default(),
              time,
              crate::test::dummy_addr(),
              Id(u16::MAX));

    let generated = step.next(Default::default(), time, crate::test::dummy_addr());
    assert_eq!(generated, Id(1))
  }

  #[test]
  fn next_should_generate_in_gap_when_smallest_1_and_largest_max() {
    let mut step = alloc::ProvisionIds::<P, ()>::default();
    let time = ClockMock::instant(0);

    step.seen(Default::default(), time, crate::test::dummy_addr(), Id(1));
    step.seen(Default::default(), time, crate::test::dummy_addr(), Id(2));
    step.seen(Default::default(), time, crate::test::dummy_addr(), Id(3));
    step.seen(Default::default(), time, crate::test::dummy_addr(), Id(5));
    step.seen(Default::default(),
              time,
              crate::test::dummy_addr(),
              Id(u16::MAX));

    let generated = step.next(Default::default(), time, crate::test::dummy_addr());
    assert_eq!(generated, Id(4))
  }

  #[test]
  fn next_should_generate_initial_id() {
    let mut step = alloc::ProvisionIds::<P, ()>::default();
    let id = step.next(Default::default(),
                       ClockMock::instant(0),
                       crate::test::dummy_addr());
    assert_eq!(id, Id(1))
  }
}
