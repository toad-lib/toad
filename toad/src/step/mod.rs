use core::convert::Infallible;

use no_std_net::SocketAddr;
use toad_msg::Token;

use crate::net::Addrd;
use crate::platform::{self, Platform};

/// # ACKing incoming messages
///
/// This step will send empty ACK messages to
/// all received CON messages (applies to both server & client flows)
pub mod ack;

/// # Buffering Responses
///
/// This step module only applies to the client flow.
///
/// [`BufferResponses`](buffer_responses::alloc::BufferResponses) ([`no_alloc`](buffer_responses::no_alloc::BufferResponses))
/// handles responses received during the client flow (polling for a response to a sent request)
///
/// If the response gotten matches the token of the sent request, nothing is done and
/// the next step will get the response.
///
/// If the response does not match the request token, and it has not seen a response to this
/// request yet, then the response is stored in the buffer and `WouldBlock` is yielded.
///
/// If the response does not match the request token, and it has buffered a response to this
/// request, then the response is stored in the buffer and the matching response is taken out of the buffer.
pub mod buffer_responses;

/// # Parsing step
///
/// This step is responsible for initiating the Step pipe
/// by reading the platform's [`Snapshot`](crate::platform::Snapshot) for
/// a dgram received from an external source.
///
/// This step does no filtering whatsoever and _just_ parses the dgram
/// into a [`toad_msg::Message`] then into a [`Req`](crate::req::Req) or [`Resp`](crate::resp::Resp).
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
/// use toad::net::Addrd;
/// use toad::platform::{Effect, Message, Snapshot, Std};
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
///
///   fn message_sent(&mut self, msg: &Addrd<Message<Std>>) -> Result<(), Self::Error> {
///     Ok(())
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
///
///   fn message_sent(&mut self, msg: &Addrd<Message<Std>>) -> Result<(), Self::Error> {
///     Ok(())
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
/// let snap = Snapshot::<Std>::new(toad::std::Clock::new().try_now().unwrap(),
///                                 toad::net::Addrd(Default::default(), addr));
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
/// use no_std_net::SocketAddr;
/// use toad::net::Addrd;
/// use toad::platform::{Effect, Message, Snapshot, Std};
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
///   # fn message_sent(&mut self, msg: &Addrd<Message<Std>>) -> Result<(), Self::Error> {
///   #   panic!()
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

  /// A message has been sent over the wire
  fn message_sent(&mut self, msg: &Addrd<platform::Message<P>>) -> Result<(), Self::Error>;
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

  fn message_sent(&mut self, _: &Addrd<platform::Message<P>>) -> Result<(), Self::Error> {
    Ok(())
  }
}

#[cfg(test)]
pub mod test {
  use embedded_time::Clock;

  use super::*;
  use crate::test;
  use crate::test::ClockMock;

  pub fn default_snapshot() -> platform::Snapshot<test::Platform> {
    platform::Snapshot::new(ClockMock::new().try_now().unwrap(),
                            crate::net::Addrd(Default::default(), crate::test::dummy_addr()))
  }

  #[macro_export]
  macro_rules! dummy_step {
    ({Step<PollReq = $poll_req_ty:ty, PollResp = $poll_resp_ty:ty, Error = $error_ty:ty>}) => {
      #[derive(Default)]
      struct Dummy;

      static mut POLL_REQ_MOCK: Option<::nb::Result<$poll_req_ty, $error_ty>> = None;
      static mut POLL_RESP_MOCK: Option<Box<dyn Fn() -> Option<::nb::Result<$poll_resp_ty,
                                                                            $error_ty>>>> = None;
      unsafe {
        POLL_RESP_MOCK = Some(Box::new(|| None));
      }

      impl Step<$crate::test::Platform> for Dummy {
        type PollReq = $poll_req_ty;
        type PollResp = $poll_resp_ty;
        type Error = $error_ty;

        fn poll_req(&mut self,
                    _: &$crate::platform::Snapshot<$crate::test::Platform>,
                    _: &mut <$crate::test::Platform as $crate::platform::Platform>::Effects)
                    -> $crate::step::StepOutput<Self::PollReq, Self::Error> {
          unsafe { POLL_REQ_MOCK.clone() }
        }

        fn poll_resp(&mut self,
                     _: &$crate::platform::Snapshot<$crate::test::Platform>,
                     _: &mut <$crate::test::Platform as $crate::platform::Platform>::Effects,
                     _: toad_msg::Token,
                     _: no_std_net::SocketAddr)
                     -> $crate::step::StepOutput<Self::PollResp, ()> {
          unsafe { POLL_RESP_MOCK.as_ref().unwrap()() }
        }

        fn message_sent(&mut self,
                        _: &Addrd<$crate::platform::Message<$crate::test::Platform>>)
                        -> Result<(), Self::Error> {
          Ok(())
        }
      }
    };
  }

  #[macro_export]
  macro_rules! test_step_when {
    (
      poll_req_mock = $poll_req_mock:expr,
      poll_resp_mock = $poll_resp_mock:expr,
      effects = $effects:expr,
      snapshot = $snapshot:expr,
      token = $token:expr,
      addr = $addr:expr,
      when (inner.poll_req => {$inner_step_returns:expr})
    ) => {
      *$poll_req_mock = $inner_step_returns
    };
    (
      poll_req_mock = $poll_req_mock:expr,
      poll_resp_mock = $poll_resp_mock:expr,
      effects = $effects_mut:expr,
      snapshot = $snapshot:expr,
      token = $token:expr,
      addr = $addr:expr,
      when (effects = {$effects:expr})
    ) => {
      *$effects_mut = $effects
    };
    (
      poll_req_mock = $poll_req_mock:expr,
      poll_resp_mock = $poll_resp_mock:expr,
      effects = $effects:expr,
      snapshot = $snapshot:expr,
      token = $token:expr,
      addr = $addr:expr,
      when (inner.poll_resp => {$inner_step_returns:expr})
    ) => {
      *$poll_resp_mock = Some(Box::new(|| $inner_step_returns))
    };
    (
      poll_req_mock = $poll_req_mock:expr,
      poll_resp_mock = $poll_resp_mock:expr,
      effects = $effects:expr,
      snapshot = $snapshot:expr,
      token = $token:expr,
      addr = $addr:expr,
      when (inner.poll_resp = {$poll_resp_fake:expr})
    ) => {
      *$poll_resp_mock = Some(Box::new($poll_resp_fake))
    };
    (
      poll_req_mock = $poll_req_mock:expr,
      poll_resp_mock = $poll_resp_mock:expr,
      effects = $effects:expr,
      snapshot = $snapshot_mut:expr,
      token = $token:expr,
      addr = $addr:expr,
      when (snapshot = {$snapshot:expr})
    ) => {
      *$snapshot_mut = $snapshot
    };
    (
      poll_req_mock = $poll_req_mock:expr,
      poll_resp_mock = $poll_resp_mock:expr,
      effects = $effects:expr,
      snapshot = $snapshot_mut:expr,
      token = $token_mut:expr,
      addr = $addr:expr,
      when (poll_resp_token = {$token:expr})
    ) => {
      *$token_mut = $token
    };
    (
      poll_req_mock = $poll_req_mock:expr,
      poll_resp_mock = $poll_resp_mock:expr,
      effects = $effects:expr,
      snapshot = $snapshot_mut:expr,
      token = $token:expr,
      addr = $addr_mut:expr,
      when (poll_resp_addr = {$addr:expr})
    ) => {
      *$addr_mut = $addr
    };
  }

  #[macro_export]
  macro_rules! test_step_expect {
    (
      step: $step_ty:ty = $step:expr,
      snap = $snap:expr,
      effects = $effects:expr,
      token = $token:expr,
      addr = $addr:expr,
      expect (poll_req(_, _) should satisfy {$assert_fn:expr})
    ) => {{
      use $crate::step::{Step, StepOutput};

      let assert_fn: Box<dyn Fn(StepOutput<<$step_ty as Step<_>>::PollReq,
                                           <$step_ty as Step<_>>::Error>)> = Box::new($assert_fn);
      assert_fn($step.poll_req($snap, $effects))
    }};
    (
      step: $step_ty:ty = $step:expr,
      snap = $snap:expr,
      effects = $effects:expr,
      token = $token:expr,
      addr = $addr:expr,
      expect (poll_resp(_, _, _, _) should satisfy {$assert_fn:expr})
    ) => {{
      use $crate::step::{Step, StepOutput};

      let assert_fn: Box<dyn Fn(StepOutput<<$step_ty as Step<_>>::PollResp,
                                           <$step_ty as Step<_>>::Error>)> = Box::new($assert_fn);
      assert_fn($step.poll_resp($snap, $effects, $token, $addr))
    }};
    (
      step: $step_ty:ty = $step:expr,
      snap = $snap:expr,
      effects = $effects:expr,
      token = $_t:expr,
      addr = $_a:expr,
      expect (poll_resp(_, _, $token:expr, $addr:expr) should satisfy {$assert_fn:expr})
    ) => {{
      use $crate::step::{Step, StepOutput};

      let assert_fn: Box<dyn Fn(StepOutput<<$step_ty as Step<_>>::PollResp,
                                           <$step_ty as Step<_>>::Error>)> = Box::new($assert_fn);
      assert_fn($step.poll_resp($snap, $effects, $token, $addr))
    }};
    (
      step: $step_ty:ty = $step:expr,
      snap = $snap:expr,
      effects = $effects:expr,
      token = $token:expr,
      addr = $addr:expr,
      expect (effects == {$expect:expr})
    ) => {
      assert_eq!($effects, &$expect)
    };
  }

  #[macro_export]
  macro_rules! test_step {
    (
      GIVEN $step:ty where $inner:ty: $inner_step:tt;
      WHEN $when_summary:ident [$($when:tt),+]
      THEN $then_summary:ident [$($expect:tt),+]
    ) => {
      paste::paste! {
        #[test]
        fn [<when_ $when_summary:lower _then_ $then_summary:lower>]() {
          #![allow(unused_mut)]
          #![allow(unused_variables)]

          use $crate::{dummy_step, test_step_when, test_step_expect, test, platform};

          dummy_step!($inner_step);

          let mut effects: <test::Platform as platform::Platform>::Effects = Default::default();
          let mut snapshot: platform::Snapshot<test::Platform> = $crate::step::test::default_snapshot();
          let mut token = ::toad_msg::Token(Default::default());
          let mut addr = test::dummy_addr();

          unsafe {
            $(
                test_step_when!(
                  poll_req_mock = &mut POLL_REQ_MOCK,
                  poll_resp_mock = &mut POLL_RESP_MOCK,
                  effects = &mut effects,
                  snapshot = &mut snapshot,
                  token = &mut token,
                  addr = &mut addr,
                  when $when
                )
            );+
          };

          let mut step = $step::default();

          $(
            test_step_expect!(
              step: $step = &mut step,
              snap = &snapshot,
              effects = &mut effects,
              token = token,
              addr = addr,
              expect $expect
            )
          );+
        }
      }
    };
  }

  pub use {dummy_step, test_step, test_step_when};
}
