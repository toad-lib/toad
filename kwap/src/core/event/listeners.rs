use kwap_msg::TryFromBytes;

use super::{Event, Eventer};
use crate::{config::{self, Config},
            resp::Resp};

/// Accepts a [`Event::RecvDgram`] and fires either [`Event::RecvMsg`] or [`Event::MsgParseError`].
///
/// # IO
/// - Fires other events
/// - Invokes [`Option.take()`] on the datagram in the [`Event::RecvDgram`].
///
/// # Panics
/// - When invoked on an event type other than RecvDgram.
/// - When an event handler took the dgram out of the event before this handler was called.
pub fn try_parse_message<Cfg: Config, Evr: Eventer<Cfg>>(ep: &Evr, ev: &mut Event<Cfg>) {
  let dgram = ev.get_mut_dgram()
                .expect("try_parse_message invoked on an event type other than RecvDgram");
  let dgram = dgram.take().expect("Dgram was already taken out of the event");

  match config::Message::<Cfg>::try_from_bytes(dgram) {
    | Ok(msg) => ep.fire(Event::RecvMsg(Some(msg))),
    | Err(e) => ep.fire(Event::MsgParseError(e)),
  }
}

/// Accepts a [`Event::RecvMsg`] and fires [`Event::RecvResp`] when that message is a response.
///
/// # IO
/// - Fires other events
/// - Invokes [`Option.take()`] on the message in the [`Event::RecvMsg`].
///
/// # Panics
/// - When invoked on an event type other than RecvMsg.
/// - When an event handler took the data out of the event before this handler was called.
pub fn resp_from_msg<Cfg: Config, Evr: Eventer<Cfg>>(ep: &Evr, ev: &mut Event<Cfg>) {
  // TODO: can these be statically guaranteed somehow?
  let msg = ev.get_mut_msg()
              .expect("resp_from_msg invoked on an event type other than RecvMsg")
              .take()
              .expect("Message was already taken out of the event");

  // TODO: Code.is_resp / Code.is_req
  if msg.code.class != 0 {
    let resp = Resp::<Cfg>::from(msg);
    ep.fire(Event::RecvResp(Some(resp)));
  }
}

/// Logs an event using println
#[cfg(any(test, not(feature = "no_std")))]
pub fn log<Cfg: Config, Evr: Eventer<Cfg>>(_: &Evr, ev: &mut Event<Cfg>) {
  println!("Event: {:?}", ev);
}
