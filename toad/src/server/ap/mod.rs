use state::{ApState, Combine, Complete, CompleteWhenHydrated, Hydrated, Unhydrated};
use toad_common::Cursor;
use toad_msg::Code;

use crate::net::Addrd;
use crate::platform::PlatformTypes;
use crate::req::Req;
use crate::todo::String1Kb;

mod inner;
/// Compile-time encoding of "completeness" of Aps
pub mod state;

pub(crate) use inner::*;

/// Record used to partially describe a [`crate::resp::Resp`]
#[non_exhaustive]
#[allow(missing_docs)]
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

/// Record used to share "hydration" across Ap states
#[non_exhaustive]
#[allow(missing_docs)]
pub struct Hydrate<P>
  where P: PlatformTypes
{
  pub req: Addrd<Req<P>>,
  pub path: Cursor<String1Kb>,
}

impl<P> Hydrate<P> where P: PlatformTypes
{
  /// Construct a [`Hydrate`] from [`Addrd`]`<`[`Req`]`>`
  pub fn from_request(req: Addrd<Req<P>>) -> Self {
    Self { path: Cursor::new(req.data().path().ok().flatten().unwrap_or("").into()),
           req }
  }
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

/// # Server Resources & Request-Response Semantics
/// `Ap` can be thought of as an extension of [`Result`] that adds new, servery, states.
///
/// ## Variants
///  * `Ok`, `Err` - [`Result::Ok`], [`Result::Err`]
///  * `OkHydrated` - this is [`Result::Ok`] with a CoAP request and partially consumed request path
///  * `Reject`, `RejectHydrated` - this has been rejected by the endpoint because a filter failed. This behaves just like `Err` but is separate because it should always be recovered. The unhydrated variant (constructed with [`Ap::reject()`] is useful for writing helper functions that exist outside of a specific server context)
///  * `Respond`, `RespondHydrated` - this request has been matched with a resource and has a response to send. This implies no other endpoints or resources will see the original request.
///
/// ## States
///  * [`Unhydrated`] - `Ap` that is just some data; is not a result that the server can act on
///  * [`Hydrated`] - `Ap` that is just some data **in the context of an incoming request**; is not a result that the server can act on
///  * [`CompleteWhenHydrated`] - `Ap` that is a result that the server can act on **once it is lifted to the context of a request**
///  * [`Complete`] - `Ap` that is a result that the server can act on. This is the final state `Ap`s should always reach
///
/// ## Type parameters
/// * `S: ApState` - compile-time state that guarantees users complete an Ap by rejecting, responding, or erroring.
/// * `P: PlatformTypes` - Decouples `Ap` from the platform being run on (e.g. heap allocation)
/// * `T` - The type in the "Ok" channel
/// * `E` - The type in the "Err" channel
///
/// ## Constructing an `Ap`
/// * [`Ap::ok`] _([`Unhydrated`])_
/// * [`Ap::ok_hydrated`] _([`Hydrated`])_
/// * [`Ap::reject`] _([`CompleteWhenHydrated`])_
/// * [`Ap::respond`] _([`CompleteWhenHydrated`])_
/// * [`Ap::err`] _([`Complete`])_
/// * [`Ap::reject_hydrated`] _([`Complete`])_
/// * [`Ap::respond_hydrated`] _([`Complete`])_
///
/// ## Destructuring an `Ap`
/// * [`Ap::try_unwrap_err`]
/// * [`Ap::try_unwrap_ok`]
/// * [`Ap::try_unwrap_ok_hydrated`]
/// * [`Ap::try_unwrap_respond`]
///
/// ## Modifying & combining `Ap`s
/// ### Changing the value (or type) in the Ok channel
/// This can be done with [`Ap::map`].
/// ```
/// use toad::server::ap::*;
/// use toad::std::{dtls, PlatformTypes as Std};
///
/// let my_ap: Ap<_, Std<dtls::Y>, String, ()> = Ap::ok(1234u32).map(|n| n.to_string());
/// assert_eq!(my_ap, Ap::ok("1234".into()));
/// ```
///
/// ### Changing the value (or type) in the Err channel
/// This can be done with [`Ap::map_err`].
/// ```
/// use toad::server::ap::*;
/// use toad::std::{dtls, PlatformTypes as Std};
///
/// #[derive(Debug)]
/// enum Error {
///   UhOh(String),
/// }
///
/// let my_ap: Ap<_, Std<dtls::Y>, (), String> =
///   Ap::err(Error::UhOh("failed to do the thing!".to_string())).map_err(|e| format!("{e:?}"));
/// assert_eq!(my_ap,
///            Ap::err("UhOh(\"failed to do the thing!\")".to_string()));
/// ```
///
/// ### Combining multiple Aps
/// [`Ap::bind`] is used to combine Aps. In practice, bind is identical to [`Result::and_then`].
///
/// The provided closure will be called when the `Ap` is in the `Ok` or `OkHydrated` channels,
/// and ignored when the `Ap` is any other state.
///
/// Hydration is **contagious**; if the closure returns an unhydrated variant ([`Ap::reject`], [`Ap::respond`], [`Ap::ok`]),
/// and `self` is [`Ap::ok_hydrated`] then the closure's return value will be hydrated before
/// continuing.
/// (e.g. self is `ok_hydrated`, closure returned `reject`, output will be `reject_hydrated`)
/// ```
/// use toad::net::Addrd;
/// use toad::req::Req;
/// use toad::server::ap::*;
/// use toad::std::{dtls, PlatformTypes as Std};
///
/// #[derive(Debug, PartialEq)]
/// enum Error {
///   UhOh(String),
/// }
///
/// let addr: no_std_net::SocketAddr = "1.1.1.1:5683".parse().unwrap();
/// let req = || Req::get("hello");
///
/// // OkHydrated.bind(Ok) => OkHydrated
/// let ok_hy_123: Ap<_, Std<dtls::Y>, u32, Error> =
///   Ap::ok_hydrated(123, Hydrate::from_request(Addrd(req(), addr)));
///
/// let ok_hy_234 = ok_hy_123.bind(|n| Ap::ok(n + 111));
///
/// assert_eq!(ok_hy_234,
///            Ap::ok_hydrated(234, Hydrate::from_request(Addrd(req(), addr))));
///
/// // OkHydrated.bind(Err) => Err
/// let ok_hy: Ap<_, Std<dtls::Y>, u32, Error> =
///   Ap::ok_hydrated(123, Hydrate::from_request(Addrd(req(), addr)));
///
/// let err: Ap<_, _, u32, Error> = ok_hy.bind(|n| Ap::err(Error::UhOh("".into())));
///
/// assert_eq!(err, Ap::err(Error::UhOh("".into())));
///
/// // RejectHydrated.bind(Err) => RejectHydrated
/// let reject: Ap<_, Std<dtls::Y>, u32, Error> = Ap::reject_hydrated(Addrd(req(), addr));
///
/// let reject_unchanged: Ap<_, _, u32, Error> = reject.bind(|n| Ap::err(Error::UhOh("".into())));
///
/// assert_eq!(reject_unchanged, Ap::reject_hydrated(Addrd(req(), addr)));
/// ```
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
  /// Construct an `Ap` in an `Ok` state
  pub fn ok(t: T) -> Self {
    Self(ApInner::Ok(t))
  }

  /// Map [`Result::Ok`] -> [`Ap::ok`], [`Result::Err`] -> [`Ap::err`]
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
  /// Construct an `Ap` that will reject the incoming
  /// request.
  ///
  /// A rejected Ap should **always** be recovered,
  /// as it signifies "this route did not match this
  /// request" and a request must always be matched with
  /// a response.
  pub fn reject() -> Self {
    Self(ApInner::Reject)
  }

  /// Construct an `Ap` that will respond to the incoming
  /// request.
  pub fn respond(r: Respond<P>) -> Self {
    Self(ApInner::Respond(r))
  }
}

impl<T, P, E> Ap<Hydrated, P, T, E>
  where P: PlatformTypes,
        E: core::fmt::Debug
{
  /// [`Ap::ok`] with a [`Hydrate`] request context
  pub fn ok_hydrated(t: T, hy: Hydrate<P>) -> Ap<Hydrated, P, T, E> {
    Ap(ApInner::OkHydrated(t, hy))
  }

  /// [`Ap::respond`] with a request context
  pub fn respond_hydrated(req: Addrd<Req<P>>, rep: Respond<P>) -> Self {
    Self(ApInner::RespondHydrated(rep, req))
  }
}

impl<P, T, E> Ap<Complete, P, T, E>
  where P: PlatformTypes,
        E: core::fmt::Debug
{
  /// Coerce the state type to any other (this only applies to `Ap`s which are known to be [`Complete`].)
  pub fn pretend<S>(self) -> Ap<S, P, T, E>
    where S: ApState
  {
    self.coerce_state()
  }

  /// [`Ap::reject`] with a request context
  pub fn reject_hydrated(req: Addrd<Req<P>>) -> Self {
    Ap(ApInner::RejectHydrated(req))
  }

  /// `Ap` in an error channel
  pub fn err(e: E) -> Self {
    Self(ApInner::Err(e))
  }

  /// Pretend this `Ap` is [`Hydrated`]. (it must be currently [`Complete`])
  pub fn pretend_hydrated(self) -> Ap<Hydrated, P, T, E> {
    self.coerce_state()
  }
}

impl<S, P, T, E> Ap<S, P, T, E>
  where P: PlatformTypes,
        S: ApState,
        E: core::fmt::Debug
{
  /// Is this [`Ap::ok`] or [`Ap::ok_hydrated`]?
  pub fn is_ok(&self) -> bool {
    match self.0 {
      | ApInner::Ok(_) | ApInner::OkHydrated(_, _) => true,
      | _ => false,
    }
  }

  /// Is this [`Ap::reject`] or [`Ap::reject_hydrated`]?
  pub fn is_rejected(&self) -> bool {
    match self.0 {
      | ApInner::Reject | ApInner::RejectHydrated(_) => true,
      | _ => false,
    }
  }

  /// Convert [`Ap::ok`] -> [`Ap::ok_hydrated`], [`Ap::reject`] -> [`Ap::reject_hydrated`],
  /// [`Ap::respond`] -> [`Ap::respond_hydrated`].
  pub fn hydrate(self, req: Addrd<Req<P>>) -> Ap<<S as Combine<Hydrated>>::Out, P, T, E> {
    match self.0 {
      | ApInner::Phantom(_) => unreachable!(),
      | ApInner::Ok(t) => Ap::ok_hydrated(t, Hydrate::from_request(req)).coerce_state(),
      | ApInner::OkHydrated(t, _) => Ap::ok_hydrated(t, Hydrate::from_request(req)).coerce_state(),
      | ApInner::Reject => Ap::reject().coerce_state(),
      | ApInner::Respond(r) => Ap::respond(r).coerce_state(),
      | ApInner::Err(e) => Ap::err(e).coerce_state(),
      | ApInner::RejectHydrated(r) => Ap::reject_hydrated(r).coerce_state(),
      | ApInner::RespondHydrated(rep, req) => Ap::respond_hydrated(req, rep).coerce_state(),
    }
  }

  /// More extreme than [`Ap::pretend_hydrated`], this will accept
  /// an `Ap` of any [`ApState`] and fix it as [`Unhydrated`].
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

  /// If this is [`Ap::respond`] or [`Ap::respond_hydrated`], unwrap and yield the data
  /// contained in it in [`Result::Ok`].
  ///
  /// If not, return `Err(self)`.
  pub fn try_unwrap_respond(self) -> Result<Respond<P>, Self> {
    match self.0 {
      | ApInner::Respond(r) | ApInner::RespondHydrated(r, _) => Ok(r),
      | other => Err(Self(other)),
    }
  }

  /// If this is [`Ap::err`], unwrap and yield the error
  /// contained in it in [`Result::Ok`].
  ///
  /// If not, return `Err(self)`.
  pub fn try_unwrap_err(self) -> Result<E, Self> {
    match self.0 {
      | ApInner::Err(e) => Ok(e),
      | other => Err(Self(other)),
    }
  }

  /// If this is [`Ap::ok`] or [`Ap::ok_hydrated`],
  /// unwrap and yield the data contained in it
  /// and return [`Result::Ok`].
  ///
  /// If not, return `Err(self)`.
  pub fn try_unwrap_ok(self) -> Result<T, Self> {
    match self.0 {
      | ApInner::OkHydrated(t, _) | ApInner::Ok(t) => Ok(t),
      | other => Err(Self(other)),
    }
  }

  /// If this is [`Ap::ok_hydrated`],
  /// unwrap and yield the data contained in it
  /// and return [`Result::Ok`].
  ///
  /// If not, return `Err(self)`.
  pub fn try_unwrap_ok_hydrated(self) -> Result<(T, Hydrate<P>), Self> {
    match self.0 {
      | ApInner::OkHydrated(t, hy) => Ok((t, hy)),
      | other => Err(Self(other)),
    }
  }

  /// Apply a function that accepts `Self` to `self`.
  ///
  /// ```
  /// use toad::server::ap::*;
  /// use toad::std::{dtls, PlatformTypes as Std};
  ///
  /// fn ok_to_err<S: state::ApState>(ap: Ap<S, Std<dtls::Y>, (), ()>) -> Ap<S, Std<dtls::Y>, (), ()> {
  ///   if ap.is_ok() {
  ///     Ap::err(()).pretend()
  ///   } else {
  ///     panic!("must be ok")
  ///   }
  /// }
  ///
  /// let ap = || Ap::<_, Std<dtls::Y>, (), ()>::ok(());
  ///
  /// // with pipe:
  /// ap().pipe(ok_to_err);
  ///
  /// // without:
  /// ok_to_err(ap());
  /// ```
  pub fn pipe<F, R>(self, f: F) -> R
    where F: FnOnce(Self) -> R
  {
    f(self)
  }

  /// If this is [`Ap::respond`] or [`Ap::respond_hydrated`],
  /// set the `etag` option for the response before sending.
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

  pub(crate) fn coerce_state<S2>(self) -> Ap<S2, P, T, E>
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

  /// Use a function `F` (`T -> B`) to transform the data contained in `Ap`.
  ///
  /// The function will only be called if this is [`Ap::ok`] or [`Ap::ok_hydrated`].
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

  /// Use a function `F` (`E -> B`) to transform the error contained in `Ap`.
  ///
  /// The function will only be called if this is [`Ap::err`].
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

  /// Use a function `F` (`T -> Ap<B, E>`) to transform the data contained in `Ap`
  /// and combine the result with self.
  ///
  /// The function will only be called if this is [`Ap::ok`] or [`Ap::ok_hydrated`].
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

  /// Shorthand for `bind`ing an Ap of unit `Ap<_, _, (), E>`
  /// and keeping the `T`.
  ///
  /// ```ignore
  /// fn do_stuff(t: &T) -> Ap<_, _, (), E> { ... }
  ///
  /// ap.bind(|t| do_stuff(&t).map(|_| t))
  /// ap.bind_discard(do_stuff)
  /// ```
  pub fn bind_discard<S2, F>(self, f: F) -> Self
    where F: for<'a> FnOnce(&'a T) -> Ap<S2, P, (), E>,
          S2: ApState,
          S: Combine<S2>
  {
    self.bind(|t| f(&t).map(|_| t)).coerce_state()
  }

  /// Silently ignore errors, mapping to [`Ap::reject`].
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
    let req = || Req::<crate::test::Platform>::get("foo");

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
