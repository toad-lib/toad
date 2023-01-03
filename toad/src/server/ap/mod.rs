use state::{ApState, Combine, Complete, CompleteWhenHydrated, Hydrated, Unhydrated};
use toad_common::Cursor;
use toad_msg::Code;

use crate::net::Addrd;
use crate::platform::PlatformTypes;
use crate::req::Req;
use crate::todo::String1Kb;

mod inner;
pub mod state;

pub(crate) use inner::*;

#[non_exhaustive]
pub struct Respond<P>
  where P: PlatformTypes
{
  pub code: Code,
  pub payload: P::MessagePayload,
  pub etag: Option<P::MessageOptionBytes>,
}

impl<P> Clone for Respond<P> where P: PlatformTypes
{
  fn clone(&self) -> Self {
    Respond { code: self.code,
              payload: self.payload.clone(),
              etag: self.etag.clone() }
  }
}

impl<P> PartialEq for Respond<P> where P: PlatformTypes
{
  fn eq(&self, other: &Self) -> bool {
    self.code == other.code && self.payload == other.payload && self.etag == other.etag
  }
}

impl<P> core::fmt::Debug for Respond<P> where P: PlatformTypes
{
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_struct("Respond")
     .field("code", &self.code)
     .field("payload", &self.payload)
     .field("etag", &self.etag)
     .finish()
  }
}

#[non_exhaustive]
pub struct Hydrate<P>
  where P: PlatformTypes
{
  pub req: Addrd<Req<P>>,
  pub path: Cursor<String1Kb>,
}

impl<P> Clone for Hydrate<P> where P: PlatformTypes
{
  fn clone(&self) -> Self {
    Hydrate { req: self.req.clone(),
              path: self.path.clone() }
  }
}

impl<P> PartialEq for Hydrate<P> where P: PlatformTypes
{
  fn eq(&self, other: &Self) -> bool {
    self.req == other.req && self.path == other.path
  }
}

impl<P> core::fmt::Debug for Hydrate<P> where P: PlatformTypes
{
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_struct("Hydrate")
     .field("req", &self.req)
     .field("path", &self.path)
     .finish()
  }
}

pub struct Ap<S, P, T, E>(pub(crate) ApInner<S, P, T, E>)
  where S: ApState,
        P: PlatformTypes;

impl<S, P, T, E> Clone for Ap<S, P, T, E>
  where S: ApState,
        P: PlatformTypes,
        E: Clone,
        T: Clone
{
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<S, P, T, E> core::fmt::Debug for Ap<S, P, T, E>
  where S: ApState,
        P: PlatformTypes,
        E: core::fmt::Debug,
        T: core::fmt::Debug
{
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_tuple("Ap").field(&self.0).finish()
  }
}

impl<S, P, T, E> PartialEq for Ap<S, P, T, E>
  where S: ApState,
        P: PlatformTypes,
        E: PartialEq,
        T: PartialEq
{
  fn eq(&self, other: &Self) -> bool {
    self.0 == other.0
  }
}

impl<P, T, E> Ap<Unhydrated, P, T, E>
  where P: PlatformTypes,
        E: core::fmt::Debug
{
  pub fn ok(t: T) -> Self {
    Self(ApInner::Ok(t))
  }

  pub fn from_result(res: Result<T, E>) -> Self {
    match res {
      | Result::Ok(t) => Self(ApInner::Ok(t)),
      | Result::Err(e) => Self(ApInner::Err(e)),
    }
  }
}

impl<P, T, Error> Ap<CompleteWhenHydrated, P, T, Error>
  where P: PlatformTypes,
        Error: core::fmt::Debug
{
  pub fn reject() -> Self {
    Self(ApInner::Reject)
  }

  pub fn respond(r: Respond<P>) -> Self {
    Self(ApInner::Respond(r))
  }
}

impl<T, P, E> Ap<Hydrated, P, T, E>
  where P: PlatformTypes,
        E: core::fmt::Debug
{
  pub fn ok_hydrated(t: T, hy: Hydrate<P>) -> Ap<Hydrated, P, T, E> {
    Ap(ApInner::OkHydrated(t, hy))
  }

  pub fn respond_hydrated(req: Addrd<Req<P>>, rep: Respond<P>) -> Self {
    Self(ApInner::RespondHydrated(rep, req))
  }
}

impl<P, T, E> Ap<Complete, P, T, E>
  where P: PlatformTypes,
        E: core::fmt::Debug
{
  pub fn reject_hydrated(req: Addrd<Req<P>>) -> Self {
    Ap(ApInner::RejectHydrated(req))
  }

  pub fn err(e: E) -> Self {
    Self(ApInner::Err(e))
  }

  pub fn pretend_hydrated(self) -> Ap<Hydrated, P, T, E> {
    self.coerce_state()
  }
}

impl<S, P, T, E> Ap<S, P, T, E>
  where P: PlatformTypes,
        S: ApState,
        E: core::fmt::Debug
{
  pub fn pretend_unhydrated(self) -> Ap<Unhydrated, P, T, E> {
    // NOTE: it's ok to change the state param to a lower
    // value because this just discards optional metadata
    //
    // e.g.
    // Complete -> Hydrated will just mean you have
    // to add a new expression that definitely completes it.
    //
    //
    // Doing the opposite (e.g. changing Hydrated -> Complete)
    // is disallowed and would break many assumptions.
    self.coerce_state()
  }

  pub fn try_unwrap_respond(self) -> Result<(), Self> {
    match self.0 {
      | ApInner::Reject | ApInner::RejectHydrated(_) => Ok(()),
      | other => Err(Self(other)),
    }
  }

  pub fn try_unwrap_reject(self) -> Result<(), Self> {
    match self.0 {
      | ApInner::Reject | ApInner::RejectHydrated(_) => Ok(()),
      | other => Err(Self(other)),
    }
  }

  pub fn try_unwrap_err(self) -> Result<E, Self> {
    match self.0 {
      | ApInner::Err(e) => Ok(e),
      | other => Err(Self(other)),
    }
  }

  pub fn try_unwrap_ok(self) -> Result<T, Self> {
    match self.0 {
      | ApInner::OkHydrated(t, _) | ApInner::Ok(t) => Ok(t),
      | other => Err(Self(other)),
    }
  }

  pub fn try_unwrap_ok_hydrated(self) -> Result<(T, Hydrate<P>), Self> {
    match self.0 {
      | ApInner::OkHydrated(t, hy) => Ok((t, hy)),
      | other => Err(Self(other)),
    }
  }

  pub fn pipe<F, R>(self, f: F) -> R
    where F: FnOnce(Self) -> R
  {
    f(self)
  }

  pub fn etag(self, etag: P::MessageOptionBytes) -> Self {
    match self.0 {
      | ApInner::Respond(Respond { code, payload, .. }) => {
        Ap::respond(Respond { code,
                              payload,
                              etag: Some(etag) }).coerce_state()
      },
      | ApInner::RespondHydrated(Respond { code, payload, .. }, req) => {
        Ap::respond_hydrated(req,
                             Respond { code,
                                       payload,
                                       etag: Some(etag) }).coerce_state()
      },
      | other => Self(other),
    }
  }

  fn coerce_state<S2>(self) -> Ap<S2, P, T, E>
    where S2: ApState
  {
    let inner = match self.0 {
      | ApInner::Phantom(_) => unreachable!(),
      | ApInner::Err(e) => ApInner::Err(e),
      | ApInner::OkHydrated(t, hy) => ApInner::OkHydrated(t, hy),
      | ApInner::Ok(t) => ApInner::Ok(t),
      | ApInner::Reject => ApInner::Reject,
      | ApInner::RejectHydrated(req) => ApInner::RejectHydrated(req),
      | ApInner::Respond(r) => ApInner::Respond(r),
      | ApInner::RespondHydrated(a, b) => ApInner::RespondHydrated(a, b),
    };

    Ap(inner)
  }

  pub fn map<F, B>(self, f: F) -> Ap<S, P, B, E>
    where F: FnOnce(T) -> B
  {
    let inner = match self.0 {
      | ApInner::Phantom(_) => unreachable!(),
      | ApInner::OkHydrated(t, hy) => ApInner::OkHydrated(f(t), hy),
      | ApInner::Ok(t) => ApInner::Ok(f(t)),
      | ApInner::Err(e) => ApInner::Err(e),
      | ApInner::Reject => ApInner::Reject,
      | ApInner::RejectHydrated(req) => ApInner::RejectHydrated(req),
      | ApInner::Respond(r) => ApInner::Respond(r),
      | ApInner::RespondHydrated(a, b) => ApInner::RespondHydrated(a, b),
    };

    Ap(inner)
  }

  pub fn map_err<F, B>(self, f: F) -> Ap<S, P, T, B>
    where F: FnOnce(E) -> B
  {
    let inner = match self.0 {
      | ApInner::Phantom(_) => unreachable!(),
      | ApInner::Err(e) => ApInner::Err(f(e)),
      | ApInner::OkHydrated(t, hy) => ApInner::OkHydrated(t, hy),
      | ApInner::Ok(t) => ApInner::Ok(t),
      | ApInner::RejectHydrated(req) => ApInner::RejectHydrated(req),
      | ApInner::Reject => ApInner::Reject,
      | ApInner::Respond(r) => ApInner::Respond(r),
      | ApInner::RespondHydrated(a, b) => ApInner::RespondHydrated(a, b),
    };

    Ap(inner)
  }

  pub fn bind<F, S2, B>(self, f: F) -> Ap<<S as state::Combine<S2>>::Out, P, B, E>
    where F: FnOnce(T) -> Ap<S2, P, B, E>,
          S2: ApState,
          S: state::Combine<S2>
  {
    let inner = match self.0 {
      | ApInner::Phantom(_) => unreachable!(),
      | ApInner::OkHydrated(t, hy) => match f(t).0 {
        | ApInner::Ok(r) => ApInner::OkHydrated(r, hy),
        | ApInner::Reject => ApInner::RejectHydrated(hy.req),
        | ApInner::Respond(rep) => ApInner::RespondHydrated(rep, hy.req),
        | other => other,
      },
      | ApInner::Ok(t) => f(t).0,
      | ApInner::Err(e) => ApInner::Err(e),
      | ApInner::Reject => ApInner::Reject,
      | ApInner::RejectHydrated(req) => ApInner::RejectHydrated(req),
      | ApInner::Respond(r) => ApInner::Respond(r),
      | ApInner::RespondHydrated(req, rep) => ApInner::RespondHydrated(req, rep),
    };

    Ap(inner).coerce_state()
  }

  pub fn bind_discard<S2, F>(self, f: F) -> Self
    where F: for<'a> FnOnce(&'a T) -> Ap<S2, P, (), E>,
          S2: ApState,
          S: Combine<S2>
  {
    self.bind(|t| f(&t).map(|_| t)).coerce_state()
  }

  pub fn reject_on_err<E2>(self) -> Ap<Unhydrated, P, T, E2> {
    let inner = match self.0 {
      | ApInner::Phantom(_) => unreachable!(),
      | ApInner::Err(_) => ApInner::Reject,
      | ApInner::OkHydrated(t, hy) => ApInner::OkHydrated(t, hy),
      | ApInner::Ok(t) => ApInner::Ok(t),
      | ApInner::Reject => ApInner::Reject,
      | ApInner::RejectHydrated(r) => ApInner::RejectHydrated(r),
      | ApInner::Respond(r) => ApInner::Respond(r),
      | ApInner::RespondHydrated(a, b) => ApInner::RespondHydrated(a, b),
    };

    Ap(inner)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::req::Req;
  use crate::resp::code;

  #[test]
  fn ap_variant_precedence() {
    type Ap<S> = super::Ap<S, crate::test::Platform, (), ()>;

    let addr = crate::test::x.x.x.x(80);
    let req = || Req::<crate::test::Platform>::get(addr, "foo");

    let ok = || Ap::ok(());
    let err = || Ap::err(());
    let ok_hy = || {
      Ap::ok_hydrated((),
                      Hydrate { req: Addrd(req(), addr),
                                path: Cursor::new("".into()) })
    };
    let reject = || Ap::reject();
    let respond = || {
      Ap::respond(Respond { code: code::CONTENT,
                            payload: "".into(),
                            etag: None })
    };
    let reject_hy = || Ap::reject_hydrated(Addrd(req(), addr));
    let respond_hy = || {
      Ap::respond_hydrated(Addrd(req(), addr),
                           Respond { code: code::CONTENT,
                                     payload: "".into(),
                                     etag: None })
    };

    macro_rules! case {
      (($a:expr) >>= ($b:expr) => $expect:expr) => {
        assert_eq!(dbg!($a().bind(|_| $b())), $expect().coerce_state());
      };
    }

    // Ok is the lowest precedence and loses to every other variant
    case!((ok) >>= (ok)         => (ok));
    case!((ok) >>= (err)        => (err));
    case!((ok) >>= (ok_hy)      => (ok_hy));
    case!((ok) >>= (reject)     => (reject));
    case!((ok) >>= (respond)    => (respond));
    case!((ok) >>= (reject_hy)  => (reject_hy));
    case!((ok) >>= (respond_hy) => (respond_hy));

    // OkHydrated will only win against Ok, and loses to (but hydrates) every other variant
    case!((ok_hy) >>= (ok)         => (ok_hy));
    case!((ok_hy) >>= (err)        => (err));
    case!((ok_hy) >>= (ok_hy)      => (ok_hy));
    case!((ok_hy) >>= (reject)     => (reject_hy));
    case!((ok_hy) >>= (respond)    => (respond_hy));
    case!((ok_hy) >>= (reject_hy)  => (reject_hy));
    case!((ok_hy) >>= (respond_hy) => (respond_hy));

    // Bind is skipped on Err; loses to nothing
    case!((err) >>= (ok)         => (err));
    case!((err) >>= (err)        => (err));
    case!((err) >>= (ok_hy)      => (err));
    case!((err) >>= (reject)     => (err));
    case!((err) >>= (respond)    => (err));
    case!((err) >>= (reject_hy)  => (err));
    case!((err) >>= (respond_hy) => (err));

    // Bind is skipped on Respond; loses to nothing
    case!((respond) >>= (ok)         => (respond));
    case!((respond) >>= (err)        => (respond));
    case!((respond) >>= (ok_hy)      => (respond));
    case!((respond) >>= (reject)     => (respond));
    case!((respond) >>= (respond)    => (respond));
    case!((respond) >>= (reject_hy)  => (respond));
    case!((respond) >>= (respond_hy) => (respond));

    // Bind is skipped on RespondHydrated; loses to nothing
    case!((respond_hy) >>= (ok)         => (respond_hy));
    case!((respond_hy) >>= (err)        => (respond_hy));
    case!((respond_hy) >>= (ok_hy)      => (respond_hy));
    case!((respond_hy) >>= (reject)     => (respond_hy));
    case!((respond_hy) >>= (respond)    => (respond_hy));
    case!((respond_hy) >>= (reject_hy)  => (respond_hy));
    case!((respond_hy) >>= (respond_hy) => (respond_hy));

    // Bind is skipped on Reject; loses to nothing
    case!((reject) >>= (ok)         => (reject));
    case!((reject) >>= (err)        => (reject));
    case!((reject) >>= (ok_hy)      => (reject));
    case!((reject) >>= (reject)     => (reject));
    case!((reject) >>= (respond)    => (reject));
    case!((reject) >>= (reject_hy)  => (reject));
    case!((reject) >>= (respond_hy) => (reject));

    // Bind is skipped on RejectHydrated; loses to nothing
    case!((reject_hy) >>= (ok)         => (reject_hy));
    case!((reject_hy) >>= (err)        => (reject_hy));
    case!((reject_hy) >>= (ok_hy)      => (reject_hy));
    case!((reject_hy) >>= (reject)     => (reject_hy));
    case!((reject_hy) >>= (respond)    => (reject_hy));
    case!((reject_hy) >>= (reject_hy)  => (reject_hy));
    case!((reject_hy) >>= (respond_hy) => (reject_hy));
  }
}
