use kwap_msg::TryFromBytes;

use super::{Event, Eventer};
use crate::config::{self, Config};

/// Accepts a [`Event::RecvDgram`] and fires either [`Event::RecvMsg`] or [`Event::MsgParseError`].
///
/// # IO
/// Invokes [`Option.take()`] on the datagram in the [`Event::RecvDgram`].
///
/// # Panics
/// - When invoked on an event type other than RecvDgram.
/// - When an event handler took the dgram out of the event before this handler was called.
pub fn try_parse_message<Cfg: Config, Evr: Eventer<Cfg>>(ep: &Evr, ev: &mut Event<Cfg>) {
  let dgram = ev.get_mut_dgram().unwrap();
  let dgram = dgram.take().unwrap();

  match config::Message::<Cfg>::try_from_bytes(dgram) {
    | Ok(msg) => ep.fire(Event::RecvMsg(msg)),
    | Err(e) => ep.fire(Event::MsgParseError(e)),
  }
}
