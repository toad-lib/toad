use core::fmt::Debug;

use kwap_msg::MessageParseError;
use no_std_net::SocketAddr;
use tinyvec::ArrayVec;

use crate::config::{Config, Message};
use crate::resp::Resp;

/// Event listeners useful across "Eventer" implemenations
pub mod listeners;

/// A type-level marker that a function may fire events.
#[derive(Debug, Clone, Copy)]
#[must_use = "EventIO must be returned or unwrapped."]
pub struct EventIO;

impl EventIO {
  /// Discard the EventIO, ignoring the fact that it is the result of firing an event.
  pub fn unwrap(self) -> () {}
}

/// A thing that emits kwap events
pub trait Eventer<Cfg: Config> {
  /// Fire an event
  fn fire(&mut self, event: Event<Cfg>) -> EventIO;

  /// Attach a listener function that will be invoked on events that match `mat`.
  fn listen(&mut self, mat: MatchEvent, listener: fn(&mut Self, &mut Event<Cfg>) -> EventIO);
}

type RecvDgramData = Option<(ArrayVec<[u8; 1152]>, SocketAddr)>;

/// A state transition for a message in the CoAP runtime
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum Event<Cfg: Config> {
  /// Received a datagram from the socket
  ///
  /// This is an option to allow mutably [`take`](Option.take)ing
  /// the bytes from the event, leaving `None` in its place.
  RecvDgram(RecvDgramData),
  /// Received a message, this should be transitioned into either "RecvResp", or "RecvReq"
  ///
  /// This is an option to allow mutably [`take`](Option.take)ing
  /// the bytes from the event, leaving `None` in its place.
  RecvMsg(Option<(Message<Cfg>, SocketAddr)>),
  /// Received a response
  ///
  /// This is an option to allow mutably [`take`](Option.take)ing
  /// the bytes from the event, leaving `None` in its place.
  RecvResp(Option<(Resp<Cfg>, SocketAddr)>),
  /// Failed to parse message from dgram
  MsgParseError(kwap_msg::MessageParseError),
}

impl<Cfg: Config> Event<Cfg> {
  /// When this is a RecvMsg event, yields a mutable reference to the bytes in the event.
  ///
  /// ```
  /// use kwap::config::{Message, Std};
  /// use kwap::core::event::Event;
  /// use no_std_net::{Ipv4Addr as Ip, SocketAddr, SocketAddrV4 as AddrV4};
  ///
  /// let addr: SocketAddr = AddrV4::new(Ip::new(0, 0, 0, 0), 1234).into();
  ///
  /// let msg = kwap::req::Req::<Std>::get("", 0, "").into();
  /// let mut ev = Event::<Std>::RecvMsg(Some((msg, addr)));
  /// let msg: &mut Option<(Message<Std>, SocketAddr)> = ev.get_mut_msg().unwrap();
  /// ```
  pub fn get_mut_msg(&mut self) -> Option<&mut Option<(Message<Cfg>, SocketAddr)>> {
    match self {
      | Self::RecvMsg(e) => Some(e),
      | _ => None,
    }
  }

  /// When this is a RecvResp event, yields a mutable reference to the bytes in the event.
  ///
  /// ```
  /// use kwap::config::{Message, Std};
  /// use kwap::core::event::Event;
  /// use kwap::resp::Resp;
  /// use no_std_net::{Ipv4Addr as Ip, SocketAddr, SocketAddrV4 as AddrV4};
  ///
  /// let addr: SocketAddr = AddrV4::new(Ip::new(0, 0, 0, 0), 1234).into();
  /// let req = kwap::req::Req::<Std>::get("", 0, "");
  /// let resp = Resp::for_request(req);
  ///
  /// let mut ev = Event::<Std>::RecvResp(Some((resp, addr)));
  /// let msg: &mut Option<(Resp<Std>, SocketAddr)> = ev.get_mut_resp().unwrap();
  /// ```
  pub fn get_mut_resp(&mut self) -> Option<&mut Option<(Resp<Cfg>, SocketAddr)>> {
    match self {
      | Self::RecvResp(e) => Some(e),
      | _ => None,
    }
  }

  /// When this is a RecvDgram event, yields a mutable reference to the bytes in the event.
  ///
  /// ```
  /// use kwap::config::{Message, Std};
  /// use kwap::core::event::Event;
  /// use kwap::resp::Resp;
  /// use no_std_net::{Ipv4Addr as Ip, SocketAddr, SocketAddrV4 as AddrV4};
  /// use tinyvec::ArrayVec;
  ///
  /// let addr: SocketAddr = AddrV4::new(Ip::new(0, 0, 0, 0), 1234).into();
  /// let mut ev = Event::<Std>::RecvDgram(Some((ArrayVec::default(), addr)));
  /// let msg: &mut Option<(ArrayVec<[u8; 1152]>, SocketAddr)> = ev.get_mut_dgram().unwrap();
  /// ```
  pub fn get_mut_dgram(&mut self) -> Option<&mut RecvDgramData> {
    match self {
      | Self::RecvDgram(e) => Some(e),
      | _ => None,
    }
  }

  /// Extract the MessageParseError when this is a MsgParseError event.
  ///
  /// ```
  /// use kwap::config::{Message, Std};
  /// use kwap::core::event::Event;
  /// use kwap::resp::Resp;
  /// use kwap_msg::MessageParseError;
  ///
  /// let mut ev = Event::<Std>::MsgParseError(MessageParseError::UnexpectedEndOfStream);
  /// let msg: &MessageParseError = ev.get_msg_parse_error().unwrap();
  /// ```
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
/// use kwap::config::Std;
/// use kwap::core::event::{Event, MatchEvent};
///
/// static mut LOG_ERRS_WAS_CALLED: bool = false;
///
/// fn log_errs(e: Event<Std>) {
///   eprintln!("error parsing message: {:?}", e.get_msg_parse_error().unwrap());
///   unsafe {
///     LOG_ERRS_WAS_CALLED = true;
///   }
/// }
///
/// fn listen(e: MatchEvent, f: fn(e: Event<Std>)) {
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
  /// Match any event, defer filtering to the handler
  ///
  /// This is discouraged and should only be used when a handler
  /// _needs_ to handle multiple types of events.
  All,
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
  ///   let ev = Event::<kwap::config::Std>::MsgParseError(UnexpectedEndOfStream);
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
  ///   assert!(!MatchEvent::RecvDgram.matches(&ev))
  /// }
  /// ```
  pub fn matches<Cfg: Config>(&self, event: &Event<Cfg>) -> bool {
    match *self {
      | Self::All => true,
      | Self::MsgParseError => matches!(event, Event::MsgParseError(_)),
      | Self::RecvDgram => matches!(event, Event::RecvDgram(_)),
      | Self::RecvMsg => matches!(event, Event::RecvMsg(_)),
      | Self::RecvResp => matches!(event, Event::RecvResp(_)),
    }
  }
}
