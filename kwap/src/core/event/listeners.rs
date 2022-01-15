use kwap_msg::TryFromBytes;

use super::{Event, Eventer};
use crate::config::{self, Config};
use crate::resp::Resp;

/// Accepts a [`Event::RecvDgram`] and fires either [`Event::RecvMsg`] or [`Event::MsgParseError`].
///
/// # IO
/// - Fires other events
/// - Invokes [`Option.take()`] on the datagram in the [`Event::RecvDgram`].
///
/// # Panics
/// - When invoked on an event type other than RecvDgram.
/// - When an event handler took the dgram out of the event before this handler was called.
pub fn try_parse_message<Cfg: Config, Evr: Eventer<Cfg>>(ep: &mut Evr, ev: &mut Event<Cfg>) {
  let data = ev.get_mut_dgram()
               .expect("try_parse_message invoked on an event type other than RecvDgram");
  let (dgram, addr) = data.take().expect("Dgram was already taken out of the event");

  match config::Message::<Cfg>::try_from_bytes(dgram) {
    | Ok(msg) => ep.fire(Event::<Cfg>::RecvMsg(Some((msg, addr)))),
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
pub fn resp_from_msg<Cfg: Config, Evr: Eventer<Cfg>>(ep: &mut Evr, ev: &mut Event<Cfg>) {
  // TODO: can these be statically guaranteed somehow?
  let msg = ev.get_mut_msg()
              .expect("resp_from_msg invoked on an event type other than RecvMsg");

  // TODO: Code.is_resp / Code.is_req
  if msg.as_ref().map(|(m, _)| m.code.class > 1) == Some(true) {
    let (msg, addr) = ev.get_mut_msg()
                        .unwrap()
                        .take()
                        .expect("Message was already taken out of the event");
    let resp = Resp::<Cfg>::from(msg);
    ep.fire(Event::RecvResp(Some((resp, addr))));
  }
}

/// Logs an event using println
#[cfg(any(test, not(feature = "no_std")))]
pub fn log<Cfg: Config, Evr: Eventer<Cfg>>(_: &mut Evr, ev: &mut Event<Cfg>) {
  println!("Event: {:?}", ev);
}

#[cfg(test)]
mod tests {
  use std::cell::RefCell;

  use kwap_msg::TryIntoBytes;
  use no_std_net::{Ipv4Addr, SocketAddrV4};
  use tinyvec::ArrayVec;

  use super::*;
  use crate::config::{Alloc, Message};
  use crate::core::event::MatchEvent;
  use crate::req::Req;
  #[derive(Default)]
  struct MockEventer(pub RefCell<Vec<(usize, MatchEvent, fn(&mut Self, &mut Event<Alloc>))>>);

  impl MockEventer {
    fn calls(&self, mat: MatchEvent) -> usize {
      self.0.borrow().iter().find(|(_, mat_, _)| mat_ == &mat).unwrap().0
    }
  }

  impl Eventer<Alloc> for MockEventer {
    fn fire(&mut self, mut event: Event<Alloc>) {
      let ears = self.0.borrow();
      ears.iter().for_each(|(n, mat, ear)| {
                   if mat.matches(&event) {
                     unsafe {
                       let n = n as *const _ as *mut usize;
                       *n += 1usize;
                       let me_mut = (self as *const Self as *mut Self).as_mut().unwrap();
                       ear(me_mut, &mut event);
                     }
                   }
                 })
    }

    fn listen(&mut self, mat: MatchEvent, listener: fn(&mut Self, &mut Event<Alloc>)) {
      let mut ears = self.0.borrow_mut();
      ears.push((0, mat, listener));
    }
  }

  fn panic<E: Eventer<Alloc>>(_: &mut E, event: &mut Event<Alloc>) {
    panic!("{:?}", event)
  }

  fn nop<E: Eventer<Alloc>>(_: &mut E, _: &mut Event<Alloc>) {}

  #[test]
  fn try_parse_message_ok() {
    let msg = Message::<Alloc>::from(Req::<Alloc>::get("foo", 0, ""));
    let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 1234);
    let data = (msg.try_into_bytes::<ArrayVec<[u8; 1152]>>().unwrap(), addr.into());
    let mut evr = MockEventer::default();

    evr.listen(MatchEvent::RecvDgram, try_parse_message);
    evr.listen(MatchEvent::MsgParseError, panic);
    evr.listen(MatchEvent::RecvMsg, nop);

    evr.fire(Event::RecvDgram(Some(data)));

    assert_eq!(evr.calls(MatchEvent::RecvDgram), 1);
    assert_eq!(evr.calls(MatchEvent::RecvMsg), 1);
    assert_eq!(evr.calls(MatchEvent::MsgParseError), 0);
  }

  #[test]
  #[should_panic]
  fn try_parse_message_panics_on_multiple_invocations() {
    let mut evr = MockEventer::default();
    let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 1234);

    // the first invocation takes the arrayvec out of the event,
    // and the second attempts to and panics
    evr.listen(MatchEvent::RecvDgram, try_parse_message);
    evr.listen(MatchEvent::RecvDgram, try_parse_message);

    evr.fire(Event::RecvDgram(Some((ArrayVec::new(), addr.into()))));
  }

  #[test]
  #[should_panic]
  fn try_parse_message_panics_on_wrong_event() {
    let msg = Message::<Alloc>::from(Req::<Alloc>::get("foo", 0, ""));
    let mut evr = MockEventer::default();
    let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 1234);

    // the first invocation takes the arrayvec out of the event,
    // and the second attempts to and panics
    evr.listen(MatchEvent::RecvMsg, try_parse_message);

    evr.fire(Event::<Alloc>::RecvMsg(Some((msg, addr.into()))));
  }

  #[test]
  fn resp_from_msg_ok() {
    let msg = Message::<Alloc>::from(Req::<Alloc>::get("foo", 0, ""));
    let bytes = msg.try_into_bytes().unwrap();
    let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 1234);
    let mut evr = MockEventer::default();

    evr.listen(MatchEvent::RecvDgram, try_parse_message);
    evr.listen(MatchEvent::MsgParseError, panic);
    evr.listen(MatchEvent::RecvMsg, nop);

    evr.fire(Event::RecvDgram(Some((bytes, addr.into()))));

    assert_eq!(evr.calls(MatchEvent::RecvDgram), 1);
    assert_eq!(evr.calls(MatchEvent::RecvMsg), 1);
    assert_eq!(evr.calls(MatchEvent::MsgParseError), 0);
  }

  #[test]
  fn resp_from_msg_nops_on_code_not_response() {
    let cases = vec![Req::<Alloc>::get("foo", 0, ""), Req::<Alloc>::post("foo", 0, "")];
    let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 1234);

    for case in cases {
      let mut evr = MockEventer::default();

      evr.listen(MatchEvent::RecvMsg, resp_from_msg);
      evr.listen(MatchEvent::RecvResp, nop);

      evr.fire(Event::<Alloc>::RecvMsg(Some((case.into(), addr.clone().into()))));

      assert_eq!(evr.calls(MatchEvent::RecvResp), 0);
      assert_eq!(evr.calls(MatchEvent::RecvMsg), 1);
    }
  }

  #[test]
  fn resp_from_msg_does_not_panic_on_multiple_invocations() {
    let req = Req::<Alloc>::get("foo", 0, "");
    let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 1234);

    let mut evr = MockEventer::default();

    evr.listen(MatchEvent::RecvMsg, resp_from_msg);
    evr.listen(MatchEvent::RecvMsg, resp_from_msg);

    evr.fire(Event::<Alloc>::RecvMsg(Some((req.into(), addr.into()))));
  }

  #[test]
  #[should_panic]
  fn resp_from_msg_panics_on_wrong_event() {
    let mut evr = MockEventer::default();
    let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 1234);

    evr.listen(MatchEvent::RecvDgram, resp_from_msg);

    evr.fire(Event::RecvDgram(Some((Default::default(), addr.into()))));
  }
}
