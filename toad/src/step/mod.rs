use no_std_net::SocketAddr;
use toad_msg::Token;

use crate::net::Addrd;
use crate::platform::{self, Platform};

/// # Assign message Ids to those with Id(0)
/// * Client Flow ✓
/// * Server Flow ✓
///
/// ## Internal State
/// This step will store a buffer of the last 32 unique IDs sent and received
/// per connection.
///
/// ## Behavior
/// Whenever a message is sent with an Id of 0, the Id is replaced with a new Id
/// that has not been sent or received yet.
///
/// ## Transformation
/// None
pub mod provision_ids;

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

/// Specialized `?` operator for use in step bodies, allowing early-exit
/// for Result, Option<Result> and Option<nb::Result>.
#[macro_export]
macro_rules! _try {
  (Result; $r:expr) => {_try!(Option<Result>; Some($r))};
  (Option<Result>; $r:expr) => {_try!(Option<nb::Result>; $r.map(|r| r.map_err(nb::Error::Other)))};
  (Option<nb::Result>; $r:expr) => {match $r {
    None => return None,
    Some(Err(e)) => return Some(Err(e)),
    Some(Ok(a)) => a,
  }};
}

pub use {_try, exec_inner_step};

/// An error that can be returned by a [`Step`].
pub trait Error: core::fmt::Debug {}

impl Error for () {}

/// A step in the message-handling CoAP runtime.
///
/// See the [module documentation](crate::step) for more.
pub trait Step<P: Platform>: Default {
  /// Type that this step returns when polling for a request
  type PollReq;

  /// Type that this step returns when polling for a response
  type PollResp;

  /// Type of error that can be yielded by poll_req / poll_resp
  type Error: Error + From<<Self::Inner as Step<P>>::Error>;

  /// Inner step that will be performed before this one.
  type Inner: Step<P>;

  /// Get reference to inner step
  ///
  /// This is used by default event handler implementations
  /// to invoke the handler for the inner step.
  fn inner(&mut self) -> &mut Self::Inner;

  /// # Poll for an inbound request
  /// This corresponds to the **server** flow.
  fn poll_req(&mut self,
              snap: &platform::Snapshot<P>,
              effects: &mut P::Effects)
              -> StepOutput<Self::PollReq, Self::Error>;

  /// # Poll for an inbound response
  /// This corresponds to the **client** flow.
  fn poll_resp(&mut self,
               snap: &platform::Snapshot<P>,
               effects: &mut P::Effects,
               token: Token,
               addr: SocketAddr)
               -> StepOutput<Self::PollResp, Self::Error>;

  /// Invoked before messages are sent, allowing for internal state change & modification.
  fn before_message_sent(&mut self,
                         snap: &platform::Snapshot<P>,
                         msg: &mut Addrd<platform::Message<P>>)
                         -> Result<(), Self::Error> {
    self.inner()
        .before_message_sent(snap, msg)
        .map_err(Self::Error::from)
  }

  /// Invoked after messages are sent, allowing for internal state change.
  fn on_message_sent(&mut self,
                     snap: &platform::Snapshot<P>,
                     msg: &Addrd<platform::Message<P>>)
                     -> Result<(), Self::Error> {
    self.inner()
        .on_message_sent(snap, msg)
        .map_err(Self::Error::from)
  }
}

impl<P: Platform> Step<P> for () {
  type PollReq = ();
  type PollResp = ();
  type Error = ();
  type Inner = ();

  fn inner(&mut self) -> &mut Self::Inner {
    panic!("Step.inner invoked for unit (). This is incorrect and would likely cause recursion without return")
  }

  fn poll_req(&mut self,
              _: &platform::Snapshot<P>,
              _: &mut <P as Platform>::Effects)
              -> StepOutput<(), ()> {
    None
  }

  fn poll_resp(&mut self,
               _: &platform::Snapshot<P>,
               _: &mut <P as Platform>::Effects,
               _: Token,
               _: SocketAddr)
               -> StepOutput<(), ()> {
    None
  }

  fn before_message_sent(&mut self,
                         _: &platform::Snapshot<P>,
                         _: &mut Addrd<platform::Message<P>>)
                         -> Result<(), Self::Error> {
    Ok(())
  }

  fn on_message_sent(&mut self,
                     _: &platform::Snapshot<P>,
                     _: &Addrd<platform::Message<P>>)
                     -> Result<(), Self::Error> {
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
    platform::Snapshot { time: ClockMock::new().try_now().unwrap(),
                         recvd_dgram: crate::net::Addrd(Default::default(),
                                                        crate::test::dummy_addr()),
                         config: crate::config::Config::default().into() }
  }

  #[macro_export]
  macro_rules! dummy_step {
    ({Step<PollReq = $poll_req_ty:ty, PollResp = $poll_resp_ty:ty, Error = $error_ty:ty>}) => {
      use $crate::net::Addrd;
      use $crate::{platform, step, test};

      #[derive(Default)]
      struct Dummy(());

      static mut POLL_REQ_MOCK: Option<::nb::Result<$poll_req_ty, $error_ty>> = None;
      static mut POLL_RESP_MOCK: Option<Box<dyn Fn() -> Option<::nb::Result<$poll_resp_ty,
                                                                            $error_ty>>>> = None;
      static mut ON_MESSAGE_SENT_MOCK: Option<Box<dyn Fn(&platform::Snapshot<test::Platform>,
                                                           &Addrd<test::Message>)
                                                           -> Result<(), $error_ty>>> = None;
      static mut BEFORE_MESSAGE_SENT_MOCK:
        Option<Box<dyn Fn(&platform::Snapshot<test::Platform>,
                          &mut Addrd<test::Message>) -> Result<(), $error_ty>>> = None;

      unsafe {
        POLL_RESP_MOCK = Some(Box::new(|| None));
        ON_MESSAGE_SENT_MOCK = Some(Box::new(|_, _| Ok(())));
        BEFORE_MESSAGE_SENT_MOCK = Some(Box::new(|_, _| Ok(())));
      }

      impl Step<test::Platform> for Dummy {
        type PollReq = $poll_req_ty;
        type PollResp = $poll_resp_ty;
        type Error = $error_ty;
        type Inner = ();

        fn inner(&mut self) -> &mut () {
          &mut self.0
        }

        fn poll_req(&mut self,
                    _: &platform::Snapshot<test::Platform>,
                    _: &mut <test::Platform as platform::Platform>::Effects)
                    -> step::StepOutput<Self::PollReq, Self::Error> {
          unsafe { POLL_REQ_MOCK.clone() }
        }

        fn poll_resp(&mut self,
                     _: &platform::Snapshot<test::Platform>,
                     _: &mut <test::Platform as platform::Platform>::Effects,
                     _: toad_msg::Token,
                     _: no_std_net::SocketAddr)
                     -> step::StepOutput<Self::PollResp, ()> {
          unsafe { POLL_RESP_MOCK.as_ref().unwrap()() }
        }

        fn before_message_sent(&mut self,
                               snap: &platform::Snapshot<test::Platform>,
                               msg: &mut Addrd<test::Message>)
                               -> Result<(), Self::Error> {
          unsafe { BEFORE_MESSAGE_SENT_MOCK.as_ref().unwrap()(snap, msg) }
        }

        fn on_message_sent(&mut self,
                           snap: &platform::Snapshot<test::Platform>,
                           msg: &Addrd<test::Message>)
                           -> Result<(), Self::Error> {
          unsafe { ON_MESSAGE_SENT_MOCK.as_ref().unwrap()(snap, msg) }
        }
      }
    };
  }

  #[macro_export]
  macro_rules! test_step_when {
    (
      poll_req_mock = $poll_req_mock:expr,
      poll_resp_mock = $poll_resp_mock:expr,
      before_message_sent_mock = $before_message_sent_mock:expr,
      on_message_sent_mock = $on_message_sent_mock:expr,
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
      before_message_sent_mock = $before_message_sent_mock:expr,
      on_message_sent_mock = $on_message_sent_mock:expr,
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
      before_message_sent_mock = $before_message_sent_mock:expr,
      on_message_sent_mock = $on_message_sent_mock:expr,
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
      before_message_sent_mock = $before_message_sent_mock:expr,
      on_message_sent_mock = $on_message_sent_mock:expr,
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
      before_message_sent_mock = $before_message_sent_mock:expr,
      on_message_sent_mock = $on_message_sent_mock:expr,
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
      before_message_sent_mock = $before_message_sent_mock:expr,
      on_message_sent_mock = $on_message_sent_mock:expr,
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
      before_message_sent_mock = $before_message_sent_mock:expr,
      on_message_sent_mock = $on_message_sent_mock:expr,
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
      before_message_sent_mock = $before_message_sent_mock:expr,
      on_message_sent_mock = $on_message_sent_mock:expr,
      effects = $effects:expr,
      snapshot = $snapshot_mut:expr,
      token = $token:expr,
      addr = $addr_mut:expr,
      when (inner.before_message_sent = {$before_message_sent:expr})
    ) => {
      *$before_message_sent_mock = Some(Box::new($before_message_sent))
    };
    (
      poll_req_mock = $poll_req_mock:expr,
      poll_resp_mock = $poll_resp_mock:expr,
      before_message_sent_mock = $before_message_sent_mock:expr,
      on_message_sent_mock = $on_message_sent_mock:expr,
      effects = $effects:expr,
      snapshot = $snapshot_mut:expr,
      token = $token:expr,
      addr = $addr_mut:expr,
      when (inner.on_message_sent = {$on_message_sent:expr})
    ) => {
      *$on_message_sent_mock = Some(Box::new($on_message_sent))
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
      expect (before_message_sent($msg:expr) should satisfy {$assert_fn:expr})
    ) => {{
      use $crate::step::Step;

      let assert_fn: Box<dyn Fn(Result<(), <$step_ty as Step<_>>::Error>)> = Box::new($assert_fn);
      assert_fn($step.before_message_sent(&mut $msg))
    }};
    (
      step: $step_ty:ty = $step:expr,
      snap = $snap:expr,
      effects = $effects:expr,
      token = $token:expr,
      addr = $addr:expr,
      expect (on_message_sent(_, $msg:expr) should satisfy {$assert_fn:expr})
    ) => {{
      use $crate::step::Step;

      let assert_fn: Box<dyn Fn(Result<(), <$step_ty as Step<_>>::Error>)> = Box::new($assert_fn);
      assert_fn($step.on_message_sent($snap, &$msg))
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
    (
      step: $step_ty:ty = $step:expr,
      snap = $snap:expr,
      effects = $effects:expr,
      token = $token:expr,
      addr = $addr:expr,
      expect (before_message_sent(_, $msg:expr) should be ok with {$f:expr})
    ) => {{
      let mut msg = $msg;
      $step.before_message_sent(&$snap, &mut msg).unwrap();
      let f: Box<dyn Fn($crate::net::Addrd<$crate::test::Message>)> = Box::new($f);
      f(msg)
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

          use $crate::{dummy_step, test_step_when, test_step_expect};

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
                  before_message_sent_mock = &mut BEFORE_MESSAGE_SENT_MOCK,
                  on_message_sent_mock = &mut ON_MESSAGE_SENT_MOCK,
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
