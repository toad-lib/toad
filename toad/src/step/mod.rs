use core::convert::Infallible;

use no_std_net::SocketAddr;
use toad_msg::Token;

use crate::platform::{self, Platform};

/// TODO
pub mod ack;

/// # Parsing step
/// This module contains types representing the step of the
/// CoAP message lifecycle where UDP datagrams enter and are
/// parsed as CoAP messages.
///
/// ```text
/// Dgram --> Message --> Req
///        |           |
///        -> Error    -> Resp
/// ```
pub mod parse;

/// ```text
///             None -> "You may run, the step may have done nothing or just performed some effects"
///         Some(Ok) -> "You may run, the step yielded a T that could be transformed or discarded"
///        Some(Err) -> "You should not run, something unrecoverable happened"
/// Some(WouldBlock) -> "You may run, but we should all wait until the resource would no longer block"
/// ```
pub type StepOutput<T, E> = Option<nb::Result<T, E>>;

/// Macro to execute inner steps,
/// converting the `Option<nb::Result<T, E>>` to `Option<T>`
/// by returning the inner step's Errors & WouldBlock
///
/// ```
/// use embedded_time::Clock;
/// use no_std_net::SocketAddr;
/// use toad::platform::{Effect, Snapshot, Std};
/// use toad::step::{exec_inner_step, Step, StepOutput};
///
/// #[derive(Default)]
/// struct Inner;
/// impl Step<Std> for Inner {
///   type PollReq = ();
///   type PollResp = ();
///   type Error = ();
///
///   fn poll_req(&mut self,
///               snap: &Snapshot<Std>,
///               effects: &mut Vec<Effect<Std>>)
///               -> StepOutput<Self::PollReq, Self::Error> {
///     Some(Err(nb::Error::Other(())))
///   }
///
///   fn poll_resp(&mut self,
///                snap: &Snapshot<Std>,
///                effects: &mut Vec<Effect<Std>>,
///                token: toad_msg::Token,
///                addr: SocketAddr)
///                -> StepOutput<Self::PollResp, Self::Error> {
///     Some(Err(nb::Error::Other(())))
///   }
/// }
///
/// #[derive(Default)]
/// struct MyStep<Inner>(Inner);
///
/// #[derive(Debug, PartialEq)]
/// enum MyError<E> {
///   MyStepMessedUp,
///   InnerStepMessedUp(E),
/// }
///
/// impl<E: toad::step::Error> toad::step::Error for MyError<E> {}
///
/// impl<Inner: Step<Std>> Step<Std> for MyStep<Inner> {
///   type PollReq = ();
///   type PollResp = ();
///   type Error = MyError<Inner::Error>;
///
///   fn poll_req(&mut self,
///               snap: &Snapshot<Std>,
///               effects: &mut Vec<Effect<Std>>)
///               -> StepOutput<Self::PollReq, Self::Error> {
///     exec_inner_step!(self.0.poll_req(snap, effects), MyError::InnerStepMessedUp);
///
///     panic!("macro didn't return Inner's error");
///   }
///
///   fn poll_resp(&mut self,
///                snap: &Snapshot<Std>,
///                effects: &mut Vec<Effect<Std>>,
///                token: toad_msg::Token,
///                addr: SocketAddr)
///                -> StepOutput<Self::PollResp, Self::Error> {
///     exec_inner_step!(self.0.poll_resp(snap, effects, token, addr),
///                      MyError::InnerStepMessedUp);
///
///     panic!("macro didn't return Inner's error");
///   }
/// }
///
/// let token = toad_msg::Token(Default::default());
///
/// let addr: SocketAddr = {
///   // 192.168.0.1:8080
/// # use no_std_net::*;
/// # SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(192, 168, 0, 1), 8080))
/// };
///
/// let snap = Snapshot::<Std> { recvd_dgram: toad::net::Addrd(Default::default(), addr),
///                              time: toad::std::Clock::new().try_now().unwrap() };
///
/// assert_eq!(MyStep(Inner).poll_req(&snap, &mut Default::default()),
///            Some(Err(nb::Error::Other(MyError::InnerStepMessedUp(())))));
/// assert_eq!(MyStep(Inner).poll_resp(&snap, &mut Default::default(), token, addr),
///            Some(Err(nb::Error::Other(MyError::InnerStepMessedUp(())))));
/// ```
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

/// An error that can be returned by a [`Step`].
pub trait Error: core::fmt::Debug {}

impl Error for Infallible {}
impl Error for () {}

/// An [`Error`] that just passes an inner step's error
/// through, for steps that are infallible but wrap fallible
/// steps.
///
/// ```
/// # use no_std_net::SocketAddr;
/// use toad::platform::{Effect, Snapshot, Std};
/// use toad::step::{PassThrough, Step, StepOutput};
///
/// #[derive(Default)]
/// struct ICantFailButInnerMight<Inner>(Inner);
///
/// impl<Inner: Step<Std>> Step<Std> for ICantFailButInnerMight<Inner> {
///   type PollReq = ();
///   type PollResp = ();
///   type Error = PassThrough<Inner::Error>;
///   # fn poll_req(&mut self,
///   #             snap: &Snapshot<Std>,
///   #             effects: &mut Vec<Effect<Std>>)
///   #             -> StepOutput<Self::PollReq, Self::Error> {
///   #   panic!();
///   # }
///   # fn poll_resp(&mut self,
///   #              snap: &Snapshot<Std>,
///   #              effects: &mut Vec<Effect<Std>>,
///   #              token: toad_msg::Token,
///   #              addr: SocketAddr)
///   #              -> StepOutput<Self::PollResp, Self::Error> {
///   #   panic!();
///   # }
/// }
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PassThrough<E>(E);

impl<E: core::fmt::Debug> core::fmt::Debug for PassThrough<E> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    self.0.fmt(f)
  }
}

impl<E: Error> Error for PassThrough<E> {}

/// A step in the message-handling CoAP runtime.
///
/// See the [module documentation](crate::step) for more.
pub trait Step<P: Platform>: Default {
  /// Type that this step returns when polling for a request
  type PollReq;

  /// Type that this step returns when polling for a response
  type PollResp;

  /// Type of error that can be yielded by poll_req / poll_resp
  type Error: Error;

  /// Poll for an inbound request
  ///
  /// (A message which we have no existing conception of)
  fn poll_req(&mut self,
              snap: &platform::Snapshot<P>,
              effects: &mut P::Effects)
              -> StepOutput<Self::PollReq, Self::Error>;

  /// Poll for an inbound response
  ///
  /// (A message which we are expecting as a direct result of a message we sent)
  fn poll_resp(&mut self,
               snap: &platform::Snapshot<P>,
               effects: &mut P::Effects,
               token: Token,
               addr: SocketAddr)
               -> StepOutput<Self::PollResp, Self::Error>;
}

/// A step that does nothing
///
/// This step is usually at the bottom / beginning of step chains.
///
/// e.g.
/// ```text
/// FilterResponses<AckRequests<Parse<Empty>>>
/// ```
/// means
/// ```text
/// Do nothing
/// then Parse datagrams
/// then Ack requests
/// then Filter responses
/// ```
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
  use crate::test;
  use crate::test::ClockMock;

  #[macro_export]
  macro_rules! dummy_step {
    (poll_req: $poll_req_ty:ty => $poll_req:expr, poll_resp: $poll_resp_ty:ty => $poll_resp:expr, error: $error_ty:ty) => {
      #[derive(Default)]
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
        and inner step {impl Step<Error = $inner_error_ty:ty, PollReq = $inner_poll_req_ty:ty, PollResp = $inner_poll_resp_ty:ty>}
        and io sequence {$ios:expr}
        and snapshot $snapshot_ident:ident {$snapshot:expr}
      WHEN
        poll_req is invoked
        and inner.poll_req returns $inner_poll_req_returns_ident:ident {$inner_poll_req_returns:expr}
      THEN
        poll_req should $expect_ident:ident {$expect:expr}
    ) => {
      paste::paste! {
        #[test]
        fn [<poll_req_should_ $expect_ident:lower _when_snap_ $snapshot_ident:lower _and_inner_returns_ $inner_poll_req_returns_ident:lower>]() {
          $crate::dummy_step!(poll_req: $inner_poll_req_ty => $inner_poll_req_returns, poll_resp: $inner_poll_resp_ty => panic!(), error: $inner_error_ty);

          let mut step = $step(Dummy);

          let snap = $snapshot;
          let mut ios = $ios;
          assert_eq!(step.poll_req(&snap, &mut ios), $expect);
        }
      }
    };
    (
      GIVEN
        this step {$step:expr}
        and inner step {impl Step<Error = $inner_error_ty:ty, PollReq = $inner_poll_req_ty:ty, PollResp = $inner_poll_resp_ty:ty>}
        and io sequence {$ios:expr}
        and snapshot $snapshot_ident:ident {$snapshot:expr}
      WHEN
        poll_req is invoked
        and inner.poll_req returns $inner_poll_req_returns_ident:ident {$inner_poll_req_returns:expr}
      THEN
        poll_req should $expect_ident:ident {$expect:expr}
        effects should $expect_effects_ident:ident {$expect_effects:expr}
    ) => {
      paste::paste! {
        #[test]
        fn [<poll_req_should_ $expect_ident:lower _and_effects_should_ $expect_effects_ident:lower _when_snap_ $snapshot_ident:lower _and_inner_returns_ $inner_poll_req_returns_ident:lower>]() {
          $crate::dummy_step!(poll_req: $inner_poll_req_ty => $inner_poll_req_returns, poll_resp: $inner_poll_resp_ty => panic!(), error: $inner_error_ty);

          let mut step = $step(Dummy);

          let snap = $snapshot;
          let mut ios = $ios;
          assert_eq!(step.poll_req(&snap, &mut ios), $expect);
          assert_eq!(ios, $expect_effects);
        }
      }
    };
    (
      GIVEN
        this step {$step:expr}
        and inner step {impl Step<Error = $inner_error_ty:ty, PollReq = $inner_poll_req_ty:ty, PollResp = $inner_poll_resp_ty:ty>}
        and io sequence {$ios:expr}
        and snapshot $snapshot_ident:ident {$snapshot:expr}
        and req had token {$token:expr}
        and req was sent to addr {$addr:expr}
      WHEN
        poll_resp is invoked
        and inner.poll_resp returns $inner_poll_resp_returns_ident:ident {$inner_poll_resp_returns:expr}
      THEN
        poll_resp should $expect_ident:ident {$expect:expr}
    ) => {
      paste::paste! {
        #[test]
        fn [<poll_resp_should_ $expect_ident:lower _when_snap_ $snapshot_ident:lower _and_inner_returns_ $inner_poll_resp_returns_ident:lower>]() {
          $crate::dummy_step!(poll_req: $inner_poll_req_ty => panic!(), poll_resp: $inner_poll_resp_ty => $inner_poll_resp_returns, error: $inner_error_ty);

          let mut step = $step(Dummy);

          let snap = $snapshot;
          let mut ios = $ios;
          assert_eq!(step.poll_resp(&snap, &mut ios, $token, $addr), $expect);
        }
      }
    };
  }

  pub use {dummy_step, test_step};
}
