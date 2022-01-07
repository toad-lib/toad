use core::fmt::Debug;

use kwap_msg::MessageParseError;
use tinyvec::ArrayVec;

use crate::{config::{Config, Message},
            resp::Resp};

/// Event listeners shared across kwap
pub mod listeners;

/// A thing that emits kwap events
pub trait Eventer<Cfg: Config> {
  /// Fire an event
  fn fire(&self, event: Event<Cfg>);

  /// Attach a listener function that will be invoked on events that match `mat`.
  fn listen(&mut self, mat: MatchEvent, listener: fn(&Self, &mut Event<Cfg>));
}

/// The core eventing system of `kwap`
///
/// This is a state machine that represents the lifecycle of messages (inbound and out)
#[derive(Debug, Clone)]
pub enum Event<Cfg: Config> {
  /// Received a datagram from the socket
  ///
  /// This is an option to allow mutably [`take`](Option.take)ing
  /// the bytes from the event, leaving `None` in its place.
  RecvDgram(Option<ArrayVec<[u8; 1152]>>),
  /// Failed to parse message from dgram
  MsgParseError(kwap_msg::MessageParseError),
  /// Received a message, this should be transitioned into either "RecvResp", or "RecvReq"
  RecvMsg(Option<Message<Cfg>>),
  /// Received a response
  RecvResp(Option<Resp<Cfg>>),
}

impl<Cfg: Config> Event<Cfg> {
  /// When this is a RecvMsg event, yields a mutable reference to the bytes in the event.
  pub fn get_mut_msg(&mut self) -> Option<&mut Option<Message<Cfg>>> {
    match self {
      | Self::RecvMsg(e) => Some(e),
      | _ => None,
    }
  }

  /// When this is a RecvResp event, yields a mutable reference to the bytes in the event.
  pub fn get_mut_resp(&mut self) -> Option<&mut Option<Resp<Cfg>>> {
    match self {
      | Self::RecvResp(e) => Some(e),
      | _ => None,
    }
  }

  /// When this is a RecvDgram event, yields a mutable reference to the bytes in the event.
  pub fn get_mut_dgram(&mut self) -> Option<&mut Option<ArrayVec<[u8; 1152]>>> {
    match self {
      | Self::RecvDgram(e) => Some(e),
      | _ => None,
    }
  }

  /// Extract the MessageParseError when this is a MsgParseError event.
  pub fn get_msg_parse_error(&self) -> Option<&MessageParseError> {
    match self {
      | Self::MsgParseError(e) => Some(e),
      | _ => None,
    }
  }
}

/// Used to compare events without creating them
///
/// ```
/// use kwap::{config::Alloc,
///            core::event::{Event, MatchEvent}};
///
/// static mut LOG_ERRS_WAS_CALLED: bool = false;
///
/// fn log_errs(e: Event<Alloc>) {
///   eprintln!("error parsing message: {:?}", e.get_msg_parse_error().unwrap());
///   unsafe {
///     LOG_ERRS_WAS_CALLED = true;
///   }
/// }
///
/// fn listen(e: MatchEvent, f: fn(e: Event<Alloc>)) {
///   // listeny things
///   # f(Event::MsgParseError(kwap_msg::MessageParseError::UnexpectedEndOfStream))
/// }
///
/// listen(MatchEvent::MsgParseError, log_errs);
///
/// unsafe { assert!(LOG_ERRS_WAS_CALLED) }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchEvent {
  /// see [`Event::RecvDgram`]
  RecvDgram,
  /// see [`Event::MsgParseError`]
  MsgParseError,
  /// see [`Event::RecvMsg`]
  RecvMsg,
  /// see [`Event::RecvResp`]
  RecvResp,
  /// Match any event
  All,
  /// Match multiple events
  Many(&'static ArrayVec<[Self; 3]>),
}

impl Default for MatchEvent {
  fn default() -> Self {
    Self::All
  }
}

impl MatchEvent {
  /// Check if an event is matched by this MatchEvent
  ///
  /// ```
  /// use tinyvec::ArrayVec;
  /// use kwap::core::event::{MatchEvent, Event};
  /// use kwap_msg::MessageParseError::UnexpectedEndOfStream;
  ///
  /// # main();
  /// fn main() {
  ///   let ev = Event::<kwap::config::Alloc>::MsgParseError(UnexpectedEndOfStream);
  ///   # static mut MANY: Option<ArrayVec<[MatchEvent; 3]>> = None;
  ///   let many: &'static ArrayVec<[MatchEvent; 3]> =
  ///   # unsafe {
  ///   # MANY = Some(
  ///     [MatchEvent::MsgParseError, MatchEvent::RecvDgram].into_iter().collect()
  ///   # );
  ///   # MANY.as_ref().unwrap()
  ///   # };
  ///
  ///   assert!(MatchEvent::All.matches(&ev));
  ///   assert!(MatchEvent::Many(many).matches(&ev));
  ///   assert!(!MatchEvent::RecvDgram.matches(&ev))
  /// }
  /// ```
  pub fn matches<Cfg: Config>(&self, event: &Event<Cfg>) -> bool {
    match *self {
      | Self::All => true,
      | Self::Many(many) => many.iter().any(|mat| mat.matches(event)),
      | Self::MsgParseError => matches!(event, Event::MsgParseError(_)),
      | Self::RecvDgram => matches!(event, Event::RecvDgram(_)),
      | Self::RecvMsg => matches!(event, Event::RecvMsg(_)),
      | Self::RecvResp => matches!(event, Event::RecvResp(_)),
    }
  }
}
