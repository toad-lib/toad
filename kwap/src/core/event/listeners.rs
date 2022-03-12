use kwap_msg::TryFromBytes;

use super::{Event, EventIO, Eventer, MatchEvent};
use crate::config::{self, Config};
use crate::core::{Context, Error, ErrorKind};
use crate::req::Req;
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
pub fn try_parse_message<Cfg: Config, Evr: Eventer<Cfg>>(ep: &mut Evr, ev: &mut Event<Cfg>) -> EventIO {
  let data = ev.get_mut_dgram()
               .expect("try_parse_message invoked on an event type other than RecvDgram");
  let (dgram, addr) = data.take().expect("Dgram was already taken out of the event");

  match config::Message::<Cfg>::try_from_bytes(dgram) {
    | Ok(msg) => ep.fire(Event::<Cfg>::RecvMsg(Some((msg, addr)))),
    | Err(e) => ep.fire(Event::Error(Error { inner: ErrorKind::FromBytes(e),
                                             msg: Some("failed to parse incoming message"),
                                             ctx: Context::ParsingMessage(addr) })),
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
pub fn try_parse_response<Cfg: Config, Evr: Eventer<Cfg>>(ep: &mut Evr, ev: &mut Event<Cfg>) -> EventIO {
  // TODO: can these be statically guaranteed somehow?
  let data = ev.get_mut_msg()
               .expect("resp_from_msg invoked on an event type other than RecvMsg");

  // TODO: Code.is_resp / Code.is_req
  match data.take() {
    | Some((msg @ kwap_msg::Message { code, .. }, addr)) if code.class > 1 => {
      let resp = Resp::<Cfg>::from(msg);
      ep.fire(Event::RecvResp(Some((resp, addr))))
    },
    | _ => EventIO,
  }
}

/// Accepts a [`Event::RecvMsg`] and fires [`Event::RecvReq`] when that message is a response.
///
/// # IO
/// - Fires other events
/// - Invokes [`Option.take()`] on the message in the [`Event::RecvMsg`].
///
/// # Panics
/// - When invoked on an event type other than RecvMsg.
/// - When an event handler took the data out of the event before this handler was called.
pub fn try_parse_request<Cfg: Config, Evr: Eventer<Cfg>>(ep: &mut Evr, ev: &mut Event<Cfg>) -> EventIO {
  // TODO: can these be statically guaranteed somehow?
  let data = ev.get_mut_msg()
               .expect("req_from_msg invoked on an event type other than RecvMsg");

  // TODO: Code.is_resp / Code.is_req
  match data.take() {
    | Some((msg @ kwap_msg::Message { code, .. }, addr)) if code.class == 0 => {
      let req = Req::<Cfg>::from(msg);
      ep.fire(Event::RecvReq(Some((req, addr))))
    },
    | _ => EventIO,
  }
}

/// Logs an event using println
#[cfg(any(test, feature = "std"))]
pub fn log<Cfg: Config, Evr: Eventer<Cfg>>(_: &mut Evr, ev: &mut Event<Cfg>) -> EventIO {
  println!("Event: {:?}", ev);
  EventIO
}

#[cfg(test)]
mod tests {
  use std::cell::RefCell;

  use kwap_msg::TryIntoBytes;
  use no_std_net::{Ipv4Addr, SocketAddrV4};
  use tinyvec::ArrayVec;

  use super::*;
  use crate::config::{Message, Std};
  use crate::core::event::MatchEvent;
  use crate::req::Req;
  #[derive(Default)]
  struct MockEventer(pub RefCell<Vec<(usize, MatchEvent, fn(&mut Self, &mut Event<Std>) -> EventIO)>>);

  impl MockEventer {
    fn calls(&self, mat: MatchEvent) -> usize {
      self.0
          .borrow()
          .iter()
          .find(|(_, mat_, _)| mat_ == &mat)
          .expect(&format!("expected a handler for {:?} but found none", mat))
          .0
    }
  }

  impl Eventer<Std> for MockEventer {
    fn fire(&mut self, mut event: Event<Std>) -> EventIO {
      let ears = self.0.borrow();
      ears.iter().for_each(|(n, mat, ear)| {
                   if mat.matches(&event) {
                     unsafe {
                       let n = n as *const _ as *mut usize;
                       *n += 1usize;
                       let me_mut = (self as *const Self as *mut Self).as_mut().unwrap();
                       ear(me_mut, &mut event).unwrap();
                     }
                   }
                 });
      EventIO
    }

    fn listen(&mut self, mat: MatchEvent, listener: fn(&mut Self, &mut Event<Std>) -> EventIO) {
      let mut ears = self.0.borrow_mut();
      ears.push((0, mat, listener));
    }
  }

  fn panic<E: Eventer<Std>>(_: &mut E, event: &mut Event<Std>) -> EventIO {
    panic!("{:?}", event)
  }

  fn nop<E: Eventer<Std>>(_: &mut E, _: &mut Event<Std>) -> EventIO {
    EventIO
  }

  #[test]
  fn try_parse_message_ok() {
    let msg = Message::<Std>::from(Req::<Std>::get("foo", 0, ""));
    let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 1234);
    let data = (msg.try_into_bytes::<ArrayVec<[u8; 1152]>>().unwrap(), addr.into());
    let mut evr = MockEventer::default();

    evr.listen(MatchEvent::RecvDgram, try_parse_message);
    evr.listen(MatchEvent::Error, panic);
    evr.listen(MatchEvent::RecvMsg, nop);

    evr.fire(Event::RecvDgram(Some(data))).unwrap();

    assert_eq!(evr.calls(MatchEvent::RecvDgram), 1);
    assert_eq!(evr.calls(MatchEvent::RecvMsg), 1);
    assert_eq!(evr.calls(MatchEvent::Error), 0);
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

    evr.fire(Event::RecvDgram(Some((ArrayVec::new(), addr.into()))))
       .unwrap();
  }

  #[test]
  #[should_panic]
  fn try_parse_message_panics_on_wrong_event() {
    let msg = Message::<Std>::from(Req::<Std>::get("foo", 0, ""));
    let mut evr = MockEventer::default();
    let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 1234);

    // the first invocation takes the arrayvec out of the event,
    // and the second attempts to and panics
    evr.listen(MatchEvent::RecvMsg, try_parse_message);

    evr.fire(Event::<Std>::RecvMsg(Some((msg, addr.into())))).unwrap();
  }

  #[test]
  fn resp_from_msg_ok() {
    let req = Req::<Std>::get("foo", 0, "");
    let resp = Resp::for_request(req);
    let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 1234);
    let mut evr = MockEventer::default();

    evr.listen(MatchEvent::RecvMsg, try_parse_response);
    evr.listen(MatchEvent::RecvResp, nop);

    evr.fire(Event::<Std>::RecvMsg(Some((resp.into(), addr.into()))))
       .unwrap();

    assert_eq!(evr.calls(MatchEvent::RecvResp), 1);
  }

  #[test]
  fn resp_from_msg_nops_on_code_not_response() {
    let cases = vec![Req::<Std>::get("foo", 0, ""), Req::<Std>::post("foo", 0, "")];
    let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 1234);

    for case in cases {
      let mut evr = MockEventer::default();

      evr.listen(MatchEvent::RecvMsg, try_parse_response);
      evr.listen(MatchEvent::RecvResp, nop);

      evr.fire(Event::<Std>::RecvMsg(Some((case.into(), addr.into()))))
         .unwrap();

      assert_eq!(evr.calls(MatchEvent::RecvResp), 0);
      assert_eq!(evr.calls(MatchEvent::RecvMsg), 1);
    }
  }

  #[test]
  fn resp_from_msg_does_not_panic_on_multiple_invocations() {
    let req = Req::<Std>::get("foo", 0, "");
    let resp = Resp::for_request(req);
    let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 1234);

    let mut evr = MockEventer::default();

    evr.listen(MatchEvent::RecvMsg, try_parse_response);
    evr.listen(MatchEvent::RecvMsg, try_parse_response);

    evr.fire(Event::<Std>::RecvMsg(Some((resp.into(), addr.into()))));
  }

  #[test]
  fn req_from_msg_ok() {
    let req = Req::<Std>::get("foo", 0, "");
    let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 1234);
    let mut evr = MockEventer::default();

    evr.listen(MatchEvent::RecvMsg, try_parse_request);
    evr.listen(MatchEvent::RecvReq, nop);

    evr.fire(Event::<Std>::RecvMsg(Some((req.into(), addr.into()))));

    assert_eq!(evr.calls(MatchEvent::RecvMsg), 1);
    assert_eq!(evr.calls(MatchEvent::RecvReq), 1);
  }

  #[test]
  fn req_from_msg_nops_on_response() {
    let cases = vec![Req::<Std>::get("foo", 0, ""), Req::<Std>::post("foo", 0, "")].into_iter()
                                                                                   .map(Resp::for_request)
                                                                                   .collect::<Vec<_>>();
    let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 1234);

    for case in cases {
      let mut evr = MockEventer::default();

      evr.listen(MatchEvent::RecvMsg, try_parse_request);
      evr.listen(MatchEvent::RecvReq, nop);

      evr.fire(Event::<Std>::RecvMsg(Some((case.into(), addr.into()))));

      assert_eq!(evr.calls(MatchEvent::RecvReq), 0);
      assert_eq!(evr.calls(MatchEvent::RecvMsg), 1);
    }
  }

  #[test]
  fn req_from_msg_does_not_panic_on_multiple_invocations() {
    let req = Req::<Std>::get("foo", 0, "");
    let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 1234);

    let mut evr = MockEventer::default();

    evr.listen(MatchEvent::RecvMsg, try_parse_request);
    evr.listen(MatchEvent::RecvMsg, try_parse_request);

    evr.fire(Event::<Std>::RecvMsg(Some((req.into(), addr.into()))))
       .unwrap();
  }

  #[test]
  #[should_panic]
  fn resp_from_msg_panics_on_wrong_event() {
    let mut evr = MockEventer::default();
    let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 1234);

    evr.listen(MatchEvent::RecvDgram, try_parse_response);

    evr.fire(Event::RecvDgram(Some((Default::default(), addr.into()))))
       .unwrap();
  }
}
