use core::convert::Infallible;

use no_std_net::SocketAddr;
use toad_msg::Token;

use crate::platform::{self, Platform};

/// TODO
pub type MessageParsing<S> = Parse<S>;

/// TODO
pub mod ack;

/// TODO
pub mod parse;

/// ```text
///             None -> "You may run, the step may have done nothing or just performed some effects"
///         Some(Ok) -> "You may run, the step yielded a T that could be transformed or discarded"
///        Some(Err) -> "You should not run, something unrecoverable happened"
/// Some(WouldBlock) -> "You may run, but we should all wait until the resource would no longer block"
/// ```
pub type StepOutput<T, E> = Option<nb::Result<T, E>>;

/// TODO
#[macro_export]
macro_rules! exec_inner_step {
  ($result:expr, $err:expr) => {
    exec_inner_step!(run_anyway_when_would_block = false, $result, $err)
  };
  (run_anyway_when_would_block = $run_anyway_when_would_block:expr, $result:expr, $err:expr) => {
    match $result {
      | None => None,
      | Some(Ok(t)) => Some(t),
      | Some(Err(nb::Error::WouldBlock)) if $run_anyway_when_would_block => None,
      | Some(Err(nb::Error::WouldBlock)) => return Some(Err(nb::Error::WouldBlock)),
      | Some(Err(nb::Error::Other(e))) => return Some(Err(nb::Error::Other($err(e)))),
    }
  };
}

pub use exec_inner_step;

use self::parse::Parse;

/// TODO
pub trait Error: core::fmt::Debug {}

impl Error for Infallible {}
impl Error for () {}

/// TODO
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PassThrough<E>(E);
impl<E: Error> Error for PassThrough<E> {}

/// TODO
pub trait Step<P: Platform> {
  /// TODO
  type PollReq;

  /// TODO
  type PollResp;

  /// TODO
  type Error: Error;

  /// TODO
  fn poll_req(&mut self,
              snap: &platform::Snapshot<P>,
              effects: &mut P::Effects)
              -> StepOutput<Self::PollReq, Self::Error>;

  /// TODO
  fn poll_resp(&mut self,
               snap: &platform::Snapshot<P>,
               effects: &mut P::Effects,
               token: Token,
               addr: SocketAddr)
               -> StepOutput<Self::PollResp, Self::Error>;
}

/// TODO
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Empty;

impl<P: Platform> Step<P> for Empty {
  type PollReq = ();
  type PollResp = ();
  type Error = Infallible;

  fn poll_req(&mut self,
              _: &platform::Snapshot<P>,
              _: &mut <P as Platform>::Effects)
              -> StepOutput<(), Infallible> {
    None
  }

  fn poll_resp(&mut self,
               _: &platform::Snapshot<P>,
               _: &mut <P as Platform>::Effects,
               _: Token,
               _: SocketAddr)
               -> StepOutput<(), Infallible> {
    None
  }
}

#[cfg(test)]
pub(self) mod test {
  use embedded_time::Clock;

  use super::*;
  use crate::net::Addrd;
  use crate::test;
  use crate::test::ClockMock;

  #[macro_export]
  macro_rules! dummy_step {
    (poll_req: $poll_req_ty:ty => $poll_req:expr, poll_resp: $poll_resp_ty:ty => $poll_resp:expr, error: $error_ty:ty) => {
      pub struct Dummy;

      impl Step<$crate::test::Platform> for Dummy {
        type PollReq = $poll_req_ty;
        type PollResp = $poll_resp_ty;
        type Error = $error_ty;

        fn poll_req(&mut self,
                    _: &$crate::platform::Snapshot<$crate::test::Platform>,
                    _: &mut <$crate::test::Platform as $crate::platform::Platform>::Effects)
                    -> $crate::step::StepOutput<Self::PollReq, Self::Error> {
          $poll_req
        }

        fn poll_resp(&mut self,
                     _: &$crate::platform::Snapshot<$crate::test::Platform>,
                     _: &mut <$crate::test::Platform as $crate::platform::Platform>::Effects,
                     _: toad_msg::Token,
                     _: no_std_net::SocketAddr)
                     -> $crate::step::StepOutput<Self::PollResp, ()> {
          $poll_resp
        }
      }
    };
  }

  pub fn default_snapshot() -> platform::Snapshot<test::Platform> {
    platform::Snapshot { time: ClockMock::new().try_now().unwrap(),
                         recvd_dgram: crate::net::Addrd(Default::default(),
                                                        crate::test::dummy_addr()) }
  }

  #[macro_export]
  macro_rules! test_step {
    (
      GIVEN
        this step {$step:expr}
        and inner step {impl Step<type Error = $inner_error_ty:ty, type PollReq = $inner_poll_req_ty:ty, type PollResp = $inner_poll_resp_ty:ty}
        and io sequence {$ios:expr}
        and snapshot $snapshot_ident:ident {$snapshot:expr}
      WHEN
        poll_req is invoked
        and inner.poll_req returns $inner_poll_req_returns_ident:ident {$inner_poll_req_returns:expr}
      THEN
        poll_req should return $expect_ident:ident {$expect:expr}
    ) => {
      paste::paste! {
        #[test]
        fn [<poll_req_should_return_ $expect_ident:lower _when_platform_state_ $snapshot_ident:lower _and_inner_returns_ $inner_poll_req_returns_ident:lower>]() {
          $crate::dummy_step!(poll_req: $inner_poll_req_ty => $inner_poll_req_returns, poll_resp: $inner_poll_resp_ty => panic!(), error: $inner_error_ty);

          let mut step = $step(Dummy);

          let snap = $snapshot;
          let mut ios = $ios;
          assert_eq!(step.poll_req(&snap, &mut ios), $expect);
        }
      }
    };
  }

  pub use {dummy_step, test_step};
}
