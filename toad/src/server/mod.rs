#![allow(missing_docs)]

use toad_common::Cursor;

use self::ap::state::{Complete, Hydrated};
use self::ap::{Ap, ApInner, Hydrate, Respond};
use crate::net::Addrd;
use crate::platform::{Message, PlatformTypes};
use crate::req::Req;
use crate::resp::Resp;
use crate::todo::String1Kb;

pub mod ap;
pub mod path;
pub mod respond;

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum Error<E> {
  PathDecodeError(core::str::Utf8Error),
  RequestInvalidType(toad_msg::Type),
  User(E),
}

#[derive(Debug)]
pub enum Run<P, E>
  where P: PlatformTypes
{
  Unmatched(Addrd<Req<P>>),
  Matched(Addrd<Message<P>>),
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
  pub fn handle(ap: Ap<Complete, P, (), E>) -> Self {
    match ap.0 {
      | ApInner::Err(e) => Self::Error(Error::User(e)),
      | ApInner::RespondHydrated(Respond { code,
                                           payload,
                                           etag, },
                                 Addrd(req, addr)) => {
        Resp::for_request(&req).map(|mut resp| {
                                 resp.set_code(code);
                                 resp.set_payload(payload);

                                 if let Some(etag) = etag {
                                   resp.set_option(4, etag);
                                 }

                                 resp
                               })
                               .map(|resp| Self::Matched(Addrd(resp.into(), addr)))
                               .unwrap_or_else(|| {
                                 Self::Error(Error::RequestInvalidType(req.msg_type()))
                               })
      },
      | ApInner::RejectHydrated(req) => Self::Unmatched(req),
      | ApInner::Respond { .. }
      | ApInner::Reject
      | ApInner::Phantom(_)
      | ApInner::Ok(_)
      | ApInner::OkHydrated { .. } => unreachable!(),
    }
  }

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

  pub fn otherwise(self) -> Result<Addrd<Message<P>>, E> {
    todo!()
  }
}

#[cfg(test)]
mod tests {
  mod compiles {
    use crate::server::{path, respond, Error, Run};
    use crate::std::{dtls, PlatformTypes as Std};

    #[allow(dead_code)]
    fn foo() {
      let _ = Run::<Std<dtls::Y>, _>::Error(Error::User(())).maybe(|a| {
                a.pipe(path::segment::check::next_equals("user"))
                 .pipe(path::segment::param::u32)
                 .bind(|(_, user_id)| respond::ok(format!("hello, user ID {}!", user_id).into()))
              });
    }
  }
}
