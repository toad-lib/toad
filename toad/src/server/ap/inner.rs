use super::state::ApState;
use super::{Hydrate, Respond};
use crate::net::Addrd;
use crate::platform::PlatformTypes;
use crate::req::Req;

pub(crate) enum ApInner<S, P, T, Error>
  where S: ApState,
        P: PlatformTypes
{
  #[allow(dead_code)]
  Phantom(S),

  // Unhydrated
  Ok(T),

  // Hydrated
  OkHydrated(T, Hydrate<P>),

  // CompleteWhenHydrated
  Reject,
  Respond(Respond<P>),

  // Complete
  Err(Error),
  RejectHydrated(Addrd<Req<P>>),
  RespondHydrated(Respond<P>, Addrd<Req<P>>),
}

impl<S, P, T, E> core::fmt::Debug for ApInner<S, P, T, E>
  where S: ApState,
        P: PlatformTypes,
        E: core::fmt::Debug,
        T: core::fmt::Debug
{
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    match self {
      | ApInner::Phantom(_) => unreachable!(),
      | ApInner::Ok(t) => f.debug_tuple("ApInner::Ok").field(&t).finish(),
      | ApInner::OkHydrated(t, hy) => f.debug_tuple("ApInner::OkHydrated")
                                       .field(&t)
                                       .field(&hy)
                                       .finish(),
      | ApInner::Reject => f.debug_struct("ApInner::Reject").finish(),
      | ApInner::Respond(r) => f.debug_tuple("ApInner::Respond").field(&r).finish(),
      | ApInner::Err(e) => f.debug_tuple("ApInner::Err").field(&e).finish(),
      | ApInner::RejectHydrated(r) => f.debug_tuple("ApInner::RejectHydrated").field(&r).finish(),
      | ApInner::RespondHydrated(req, rep) => f.debug_tuple("ApInner::RespondHydrated")
                                               .field(&req)
                                               .field(&rep)
                                               .finish(),
    }
  }
}

impl<S, P, T, E> PartialEq for ApInner<S, P, T, E>
  where S: ApState,
        P: PlatformTypes,
        E: PartialEq,
        T: PartialEq
{
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      | (ApInner::Ok(a), ApInner::Ok(b)) => a == b,
      | (ApInner::OkHydrated(ta, hya), ApInner::OkHydrated(tb, hyb)) => ta == tb && hya == hyb,
      | (ApInner::Reject, ApInner::Reject) => true,
      | (ApInner::Respond(ra), ApInner::Respond(rb)) => ra == rb,
      | (ApInner::RespondHydrated(reqa, repa), ApInner::RespondHydrated(reqb, repb)) => {
        reqa == reqb && repa == repb
      },
      | (ApInner::Err(a), ApInner::Err(b)) => a == b,
      | (ApInner::RejectHydrated(a), ApInner::RejectHydrated(b)) => a == b,
      | _ => false,
    }
  }
}

impl<S, P, T, E> Clone for ApInner<S, P, T, E>
  where S: ApState,
        P: PlatformTypes,
        E: Clone,
        T: Clone
{
  fn clone(&self) -> Self {
    match self {
      | ApInner::Phantom(_) => unreachable!(),
      | ApInner::Ok(ref t) => ApInner::Ok(t.clone()),
      | ApInner::OkHydrated(t, hy) => ApInner::OkHydrated(t.clone(), hy.clone()),
      | ApInner::Reject => ApInner::Reject,
      | ApInner::RejectHydrated(r) => ApInner::RejectHydrated(r.clone()),
      | ApInner::Respond(r) => ApInner::Respond(r.clone()),
      | ApInner::RespondHydrated(req, rep) => ApInner::RespondHydrated(req.clone(), rep.clone()),
      | ApInner::Err(e) => ApInner::Err(e.clone()),
    }
  }
}
