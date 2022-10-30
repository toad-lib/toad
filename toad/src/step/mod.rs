use core::convert::Infallible;

use no_std_net::SocketAddr;
use toad_msg::Token;

use crate::net::Addrd;
use crate::platform::{self, Platform};

/// # Send Reset to ACKs we don't recognize
/// * Client Flow ✓
/// * Server Flow ✓
///
/// ## Internal State
/// This step will store the tokens of all CONfirmable messages sent,
/// removing them as they are acknowledged.
///
/// ## Behavior
/// If an ACK is received by a client or server that does not match any
/// pending CONfirmable messages, this step will:
///  * Reply to the ACK with a Reset message
///  * Log that the ACK was ignored
///
/// ## Transformation
/// If an ACK is received by a client or server that does not match any
/// pending CONfirmable messages, this step will cause further steps
/// to ignore it by yielding None.
pub mod reset;

/// # ACK incoming messages
/// * Client Flow ✓
/// * Server Flow ✓
///
/// ## Internal State
/// None
///
/// ## Behavior
/// If a CON is received by a client or server,
/// this step will reply with an ACK.
///
/// ## Transformation
/// None
pub mod ack;

/// # Ensure clients only receive relevant response
/// * Client Flow ✓
/// * Server Flow ✗
///
/// ## Internal State
///  * Stores all responses received
///
/// ## Behavior
///  * Store incoming response
///  * If we've never seen a response matching the polled request, yield WouldBlock
///  * If we have seen exactly one matching response, pop it from the buffer & yield it
///  * If we have seen more than one matching response with different [`Type`](toad_msg::Type)s, pop & yield in this priority:
///      1. ACK
///      1. CON
///      1. NON
///      1. RESET
///
/// ## Transformation
/// None
pub mod buffer_responses;

/// # Parse messages from dgrams
/// * Client Flow ✓
/// * Server Flow ✓
///
/// ## Internal State
/// None
///
/// ## Behavior
///  * Parse dgrams from snapshot into Message
///  * Wrap Message with Req/Resp (no filtering)
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
/// ```text
/// match $result {
///   | None => None,
///   | Some(Ok(t)) => Some(t),
///   | Some(Err(nb::Error::WouldBlock)) if $run_anyway_when_would_block => None,
///   | Some(Err(nb::Error::WouldBlock)) => return Some(Err(nb::Error::WouldBlock)),
///   | Some(Err(nb::Error::Other(e))) => return Some(Err(nb::Error::Other($err(e)))),
/// }
/// ```
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
      static mut MESSAGE_SENT_MOCK:
        Option<Box<dyn Fn(&$crate::net::Addrd<$crate::platform::Message<$crate::test::Platform>>)
                          -> Result<(), $error_ty>>> = None;

      unsafe {
        POLL_RESP_MOCK = Some(Box::new(|| None));
        MESSAGE_SENT_MOCK = Some(Box::new(|_| Ok(())));
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
                        msg: &$crate::net::Addrd<$crate::platform::Message<$crate::test::Platform>>)
                        -> Result<(), Self::Error> {
          unsafe { MESSAGE_SENT_MOCK.as_ref().unwrap()(msg) }
        }
      }
    };
  }

  #[macro_export]
  macro_rules! test_step_when {
    (
      poll_req_mock = $poll_req_mock:expr,
      poll_resp_mock = $poll_resp_mock:expr,
      message_sent_mock = $message_sent_mock:expr,
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
      message_sent_mock = $message_sent_mock:expr,
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
      message_sent_mock = $message_sent_mock:expr,
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
      message_sent_mock = $message_sent_mock:expr,
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
      message_sent_mock = $message_sent_mock:expr,
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
      message_sent_mock = $message_sent_mock:expr,
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
      message_sent_mock = $message_sent_mock:expr,
      effects = $effects:expr,
      snapshot = $snapshot_mut:expr,
      token = $token:expr,
      addr = $addr_mut:expr,
      when (poll_resp_addr = {$addr:expr})
    ) => {
      *$addr_mut = $addr
    };
    (
      poll_req_mock = $poll_req_mock:expr,
      poll_resp_mock = $poll_resp_mock:expr,
      message_sent_mock = $message_sent_mock:expr,
      effects = $effects:expr,
      snapshot = $snapshot_mut:expr,
      token = $token:expr,
      addr = $addr_mut:expr,
      when (inner.message_sent = {$inner_msg_sent:expr})
    ) => {
      *$message_sent_mock = Some(Box::new($inner_msg_sent))
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
      expect (message_sent($msg:expr) should satisfy {$assert_fn:expr})
    ) => {{
      use $crate::step::Step;

      let assert_fn: Box<dyn Fn(Result<(), <$step_ty as Step<_>>::Error>)> = Box::new($assert_fn);
      assert_fn($step.message_sent(&$msg))
    }};
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
    (
      step: $step_ty:ty = $step:expr,
      snap = $snap:expr,
      effects = $effects:expr,
      token = $token:expr,
      addr = $addr:expr,
      expect (effects should satisfy {$f:expr})
    ) => {{
      let f: Box<dyn Fn(&Vec<$crate::platform::Effect<$crate::test::Platform>>)> = Box::new($f);
      f($effects)
    }};
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
                  message_sent_mock = &mut MESSAGE_SENT_MOCK,
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
