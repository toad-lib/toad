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
  if msg.code.class > 1 {
    let resp = Resp::<Cfg>::from(msg);
    ep.fire(Event::RecvResp(Some(resp)));
  }
}

/// Logs an event using println
#[cfg(any(test, not(feature = "no_std")))]
pub fn log<Cfg: Config, Evr: Eventer<Cfg>>(_: &Evr, ev: &mut Event<Cfg>) {
  println!("Event: {:?}", ev);
}

#[cfg(test)]
mod tests {
  use std::{cell::{Cell, RefCell},
            collections::HashMap};

  use kwap_msg::{Code, TryIntoBytes};
  use tinyvec::ArrayVec;

  use super::*;
  use crate::{config::{Alloc, Message},
              core::event::MatchEvent,
              req::Req};
  #[derive(Default)]
  struct MockEventer(pub RefCell<Vec<(usize, MatchEvent, fn(&Self, &mut Event<Alloc>))>>);

  impl MockEventer {
    fn calls(&self, mat: MatchEvent) -> usize {
      self.0.borrow().iter().find(|(_, mat_, _)| mat_ == &mat).unwrap().0
    }
  }

  impl Eventer<Alloc> for MockEventer {
    fn fire(&self, mut event: Event<Alloc>) {
      self.0.borrow().iter().for_each(|(n, mat, ear)| {
                              if mat.matches(&event) {
                                let n = n as *const _ as *mut usize;
                                unsafe {
                                  *n += 1usize;
                                }
                                ear(&self, &mut event);
                              }
                            })
    }

    fn listen(&mut self, mat: MatchEvent, listener: fn(&Self, &mut Event<Alloc>)) {
      let mut ears = self.0.borrow_mut();
      ears.push((0, mat, listener));
    }
  }

  fn panic<E: Eventer<Alloc>>(_: &E, event: &mut Event<Alloc>) {
    panic!("{:?}", event)
  }

  fn nop<E: Eventer<Alloc>>(_: &E, _: &mut Event<Alloc>) {}

  #[test]
  fn try_parse_message_ok() {
    let msg = Message::<Alloc>::from(Req::<Alloc>::get("foo", 0, ""));
    let bytes = msg.try_into_bytes().unwrap();
    let mut evr = MockEventer::default();

    evr.listen(MatchEvent::RecvDgram, try_parse_message);
    evr.listen(MatchEvent::MsgParseError, panic);
    evr.listen(MatchEvent::RecvMsg, nop);

    evr.fire(Event::RecvDgram(Some(bytes)));

    assert_eq!(evr.calls(MatchEvent::RecvDgram), 1);
    assert_eq!(evr.calls(MatchEvent::RecvMsg), 1);
    assert_eq!(evr.calls(MatchEvent::MsgParseError), 0);
  }

  #[test]
  #[should_panic]
  fn try_parse_message_panics_on_multiple_invocations() {
    let mut evr = MockEventer::default();

    // the first invocation takes the arrayvec out of the event,
    // and the second attempts to and panics
    evr.listen(MatchEvent::RecvDgram, try_parse_message);
    evr.listen(MatchEvent::RecvDgram, try_parse_message);

    evr.fire(Event::RecvDgram(Some(ArrayVec::new())));
  }

  #[test]
  #[should_panic]
  fn try_parse_message_panics_on_wrong_event() {
    let msg = Message::<Alloc>::from(Req::<Alloc>::get("foo", 0, ""));
    let mut evr = MockEventer::default();

    // the first invocation takes the arrayvec out of the event,
    // and the second attempts to and panics
    evr.listen(MatchEvent::RecvMsg, try_parse_message);

    evr.fire(Event::RecvMsg(Some(msg)));
  }

  #[test]
  fn resp_from_msg_ok() {
    let msg = Message::<Alloc>::from(Req::<Alloc>::get("foo", 0, ""));
    let bytes = msg.try_into_bytes().unwrap();
    let mut evr = MockEventer::default();

    evr.listen(MatchEvent::RecvDgram, try_parse_message);
    evr.listen(MatchEvent::MsgParseError, panic);
    evr.listen(MatchEvent::RecvMsg, nop);

    evr.fire(Event::RecvDgram(Some(bytes)));

    assert_eq!(evr.calls(MatchEvent::RecvDgram), 1);
    assert_eq!(evr.calls(MatchEvent::RecvMsg), 1);
    assert_eq!(evr.calls(MatchEvent::MsgParseError), 0);
  }

  #[test]
  fn resp_from_msg_nops_on_code_not_response() {
    let cases = vec![Req::<Alloc>::get("foo", 0, ""), Req::<Alloc>::post("foo", 0, "")];

    for case in cases {
      let mut evr = MockEventer::default();

      evr.listen(MatchEvent::RecvMsg, resp_from_msg);
      evr.listen(MatchEvent::RecvResp, nop);

      evr.fire(Event::RecvMsg(Some(case.into())));

      assert_eq!(evr.calls(MatchEvent::RecvResp), 0);
      assert_eq!(evr.calls(MatchEvent::RecvMsg), 1);
    }
  }

  #[test]
  #[should_panic]
  fn resp_from_msg_panics_on_multiple_invocations() {
    let req = Req::<Alloc>::get("foo", 0, "");

    let mut evr = MockEventer::default();
    // the first invocation takes the arrayvec out of the event,
    // and the second attempts to and panics
    evr.listen(MatchEvent::RecvMsg, resp_from_msg);
    evr.listen(MatchEvent::RecvMsg, resp_from_msg);

    evr.fire(Event::RecvMsg(Some(req.into())));
  }

  #[test]
  #[should_panic]
  fn resp_from_msg_panics_on_wrong_event() {
    let mut evr = MockEventer::default();

    evr.listen(MatchEvent::RecvDgram, resp_from_msg);

    evr.fire(Event::RecvDgram(Some(Default::default())));
  }
}
