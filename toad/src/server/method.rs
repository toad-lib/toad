use tinyvec::{array_vec, ArrayVec};

use super::ap::state::Hydrated;
use super::ap::Hydrate;
use super::Ap;
use crate::platform::PlatformTypes;
use crate::req::Method;

/// Reject request if the code does not match `method`
pub fn is<P, T, E>(method: Method) -> impl Fn(Ap<Hydrated, P, T, E>) -> Ap<Hydrated, P, T, E>
  where P: PlatformTypes,
        E: core::fmt::Debug
{
  is_one_of(array_vec! {_ => method})
}

/// Reject request if the code is not included in `methods`
pub fn is_one_of<P, T, E>(methods: ArrayVec<[Method; 5]>)
                          -> impl Fn(Ap<Hydrated, P, T, E>) -> Ap<Hydrated, P, T, E>
  where P: PlatformTypes,
        E: core::fmt::Debug
{
  move |ap| match ap.try_unwrap_ok_hydrated() {
    | Ok((t, h))
      if methods.iter()
                .any(|m| m.code() == h.req.data().as_ref().code) =>
    {
      Ap::ok_hydrated(t, h)
    },
    | Ok((_, Hydrate { req, .. })) => Ap::reject_hydrated(req).pretend(),
    | Err(e) => e,
  }
}

/// Reject requests that aren't [`GET`](Method::GET) requests
///
/// ```
/// use toad_msg::{Type, Message, Id, Token};
/// use toad::server::{ap::{Ap, Hydrate}, method};
/// use toad::std::{PlatformTypes as P, dtls};
/// use toad::net::Addrd;
/// use toad::req::{Req, Method};
/// # let msg = |m: Method| Message::new(Type::Con, m.code(), Id(0), Token(Default::default()));
/// # let req = |m: Method| Addrd(Req::from(msg(m)), "0.0.0.0:1234".parse().unwrap());
/// # let ap = |m: Method| Ap::ok_hydrated((), Hydrate::from_request(req(m)));
///
/// let get_request: Ap<_, P<dtls::Y>, (), ()> = /* ... */
/// # ap(Method::GET);
/// assert!(get_request.pipe(method::get).is_ok());
///
/// let post_request: Ap<_, P<dtls::Y>, (), ()> = /* ... */
/// # ap(Method::POST);
/// assert!(post_request.pipe(method::get).is_rejected());
/// ```
pub fn get<P, T, E>(ap: Ap<Hydrated, P, T, E>) -> Ap<Hydrated, P, T, E>
  where P: PlatformTypes,
        E: core::fmt::Debug
{
  ap.pipe(is(Method::GET))
}

/// Reject non-POST requests
pub fn post<P, T, E>(ap: Ap<Hydrated, P, T, E>) -> Ap<Hydrated, P, T, E>
  where P: PlatformTypes,
        E: core::fmt::Debug
{
  ap.pipe(is(Method::POST))
}

/// Reject non-PUT requests
pub fn put<P, T, E>(ap: Ap<Hydrated, P, T, E>) -> Ap<Hydrated, P, T, E>
  where P: PlatformTypes,
        E: core::fmt::Debug
{
  ap.pipe(is(Method::PUT))
}

/// Reject non-DELETE requests
pub fn delete<P, T, E>(ap: Ap<Hydrated, P, T, E>) -> Ap<Hydrated, P, T, E>
  where P: PlatformTypes,
        E: core::fmt::Debug
{
  ap.pipe(is(Method::DELETE))
}
