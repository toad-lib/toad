use core::fmt::Write;

pub use ap::Ap;
use toad_common::Cursor;
use toad_msg::{OptNumber, OptValue, MessageOptions};

use self::ap::state::{Complete, Hydrated};
use self::ap::{ApInner, Hydrate, Respond};
use crate::net::{Addrd, Socket};
use crate::platform::{Message, Platform, PlatformTypes};
use crate::req::Req;
use crate::resp::Resp;
use crate::step::Step;
use crate::todo::String1Kb;

/// Server flow applicative
pub mod ap;

/// Path manipulation
///
/// * [`segment`](path::segment)
///    * [`next()`](path::segment::next) - consume the next segment of the route & combine it with data in the `Ap`
///    * [`check`](path::segment::check)
///       * [`next_is()`](path::segment::check::next_is) - assert that the next route segment matches a predicate
///       * [`next_equals()`](path::segment::check::next_is) - assert that the next route segment equals a string
///    * [`param`](path::segment::param)
///       * [`u32()`](path::segment::param::u32) - consume the next route segment and parse as u32, rejecting the request if parsing fails.
/// * [`rest()`](path::rest) - extract the full route, skipping consumed segments & combine it with data in the `Ap`
/// * [`check`](path::check)
///    * [`rest_is()`](path::check::rest_is) - assert that the rest of the route matches a predicate
///    * [`rest_equals()`](path::check::rest_equals) - assert that the rest of the route matches a string
///    * [`ends_with()`](path::check::ends_with) - assert that the rest of the route ends with a string
pub mod path;

/// Respond to requests
pub mod respond;

/// [`Run`] errors
#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum Error<E> {
  /// Path was not valid UTF8
  PathDecodeError(core::str::Utf8Error),
  /// Request was ACK / EMPTY (these should be handled & swallowed by the toad runtime)
  RequestInvalidType(toad_msg::Type),
  /// Error of input type `E`
  Other(E),
}

/// A request was received and needs to be handled by `Run`ning your code.
///
/// This data structure allows you to declare a re-runnable block
/// of code that will be invoked with every incoming request.
///
/// ```
/// use toad::server::ap::Hydrate;
/// use toad::server::{respond, Error, Run};
/// use toad::std::{dtls, PlatformTypes as Std};
///
/// let run: Run<Std<dtls::Y>, ()> = Run::Error(Error::Other(()));
/// run.maybe(|ap| {
///      let (_, Hydrate { req, .. }) = ap.try_unwrap_ok_hydrated().unwrap();
///      if req.data().path() == Ok(Some("hello")) {
///        let name = req.data().payload_str().unwrap_or("you nameless scoundrel");
///        respond::ok(format!("hi there, {}!", name).into()).hydrate(req)
///      } else {
///        respond::respond(toad::resp::code::NOT_FOUND, [].into()).hydrate(req)
///      }
///    });
/// ```
#[derive(Debug)]
pub enum Run<P, E>
  where P: PlatformTypes
{
  /// Request has not been matched yet
  Unmatched(Addrd<Req<P>>),
  /// Request has a response
  Matched(Addrd<Message<P>>),
  /// An Error occurred
  Error(Error<E>),
}

impl<P, E> PartialEq for Run<P, E>
  where P: PlatformTypes,
        E: PartialEq
{
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      | (Self::Unmatched(a), Self::Unmatched(b)) => a == b,
      | (Self::Matched(a), Self::Matched(b)) => a == b,
      | (Self::Error(a), Self::Error(b)) => a == b,
      | _ => false,
    }
  }
}

impl<P, E> Run<P, E>
  where P: PlatformTypes,
        E: core::fmt::Debug
{
  /// Lift an [`Ap`] to [`Run`]
  pub fn handle(ap: Ap<Complete, P, (), E>) -> Self {
    match ap.0 {
      | ApInner::Err(e) => Self::Error(Error::Other(e)),
      | ApInner::RespondHydrated(Respond { code,
                                           payload,
                                           etag, },
                                 Addrd(req, addr)) => {
        let mut resp = Resp::non(&req);
        resp.set_code(code);
        resp.set_payload(payload);

        if let Some(etag) = etag {
          resp.msg_mut().add_etag(etag.as_ref()).ok();
        }

        Self::Matched(Addrd(resp.into(), addr))
      },
      | ApInner::RejectHydrated(req) => Self::Unmatched(req),
      | a @ ApInner::Respond { .. }
      | a @ ApInner::Reject
      | a @ ApInner::Phantom(_)
      | a @ ApInner::Ok(_)
      | a @ ApInner::OkHydrated { .. } => unreachable!("{a:?}"),
    }
  }

  /// Use a function to potentially respond to a request
  ///
  /// Each "maybe" branch corresponds roughly to a route / RESTful CoAP resource.
  pub fn maybe<F>(self, mut f: F) -> Self
    where F: FnMut(Ap<Hydrated, P, (), E>) -> Ap<Complete, P, (), E>
  {
    match self {
      | Run::Matched(m) => Run::Matched(m),
      | Run::Error(e) => Run::Error(e),
      | Run::Unmatched(req) => {
        req.data()
           .path()
           .map(|o| o.map(String1Kb::from).unwrap_or_default())
           .map(|path| {
             Self::handle(f(Ap::ok_hydrated((),
                                            Hydrate { req,
                                                      path: Cursor::new(path) })))
           })
           .map_err(|e| Self::Error(Error::PathDecodeError(e)))
           .unwrap_or_else(|e| e)
      },
    }
  }
}

/// Newtype wrapper of an initialization function
#[derive(Debug, Clone, Copy)]
pub struct Init<T>(pub Option<T>);

/// TODO
pub trait BlockingServer<S>: Sized + Platform<S>
  where S: Step<Self::Types, PollReq = Addrd<Req<Self::Types>>, PollResp = Addrd<Resp<Self::Types>>>
{
  /// TODO
  fn run<I, R>(&self, init: Init<I>, mut handle_request: R) -> Result<(), Error<Self::Error>>
    where I: FnMut(),
          R: FnMut(Run<Self::Types, Self::Error>) -> Run<Self::Types, Self::Error>
  {
    let mut startup_msg = String1Kb::default();
    write!(
           &mut startup_msg,
           r#"
=====================================

                       _
           __   ___.--'_`.
          ( _`.'. -   'o` )
          _\.'_'      _.-'
         ( \`. )    //\`
          \_`-'`---'\\__,
           \`        `-\
            `

  toad server up and running! 🐸
  listening on `{}`.

====================================="#,
           self.socket().local_addr()
    ).ok();

    self.log(log::Level::Info, startup_msg)
        .map_err(Error::Other)?;

    init.0.map(|mut f| f());

    loop {
      let req = nb::block!(self.poll_req()).map_err(Error::Other)?;
      match handle_request(Run::Unmatched(req)) {
        | Run::Unmatched(req) => {
          let mut msg = String1Kb::default();
          write!(&mut msg,
                 "IGNORING Request, not handled by any routes! {:?}",
                 req).ok();
          self.log(log::Level::Error, msg).map_err(Error::Other)?;

          let mut msg = String1Kb::default();
          write!(
                 &mut msg,
                 r#"
Do you need a fallback?
  server.run(|run| run.maybe(..)
                      .maybe(..)
                      .maybe(..)
                      .maybe(|ap| ap.bind(|_| respond::not_found(\"Not found!\"))))
)"#
          ).ok();
        },
        | Run::Matched(rep) => nb::block!(self.send_msg(rep.clone())).map_err(Error::Other)
                                                                     .map(|_| ())?,
        | Run::Error(e) => break Err(e),
      }
    }
  }
}

impl<S, T> BlockingServer<S> for T
  where S: Step<Self::Types, PollReq = Addrd<Req<Self::Types>>, PollResp = Addrd<Resp<Self::Types>>>,
        T: Sized + Platform<S>
{
}

#[cfg(test)]
mod tests {
  mod compiles {
    use crate::server::{path, respond, Error, Run};
    use crate::std::{dtls, PlatformTypes as Std};

    #[allow(dead_code)]
    fn foo() {
      let _ = Run::<Std<dtls::Y>, _>::Error(Error::Other(())).maybe(|a| {
                a.pipe(path::segment::check::next_equals("user"))
                 .pipe(path::segment::param::u32)
                 .bind(|(_, user_id)| respond::ok(format!("hello, user ID {}!", user_id).into()))
              });
    }
  }
}
