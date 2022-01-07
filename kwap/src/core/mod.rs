use core::{cell::RefCell, str::FromStr};

use kwap_msg::{EnumerateOptNumbers, TryIntoBytes};
use no_std_net::SocketAddrV4;
use tinyvec::ArrayVec;

/// Events used by core
pub mod event;
use event::{listeners::{resp_from_msg, try_parse_message},
            Event,
            Eventer,
            MatchEvent};

use crate::{config::{self, Config},
            req::Req,
            resp::Resp,
            socket::Socket};

/// A CoAP request/response runtime that drives client- and server-side behavior.
///
/// Defined as a state machine with state transitions ([`Event`]s).
///
/// The behavior at runtime is fully customizable, with the default behavior provided via [`Core::new()`](#method.new).
#[allow(missing_debug_implementations)]
pub struct Core<Sock: Socket, Cfg: Config> {
  sock: Sock,
  // Option for these collections provides a Default implementation,
  // which is required by ArrayVec.
  //
  // This also allows us efficiently take owned responses from the collection without reindexing the other elements.
  ears: ArrayVec<[Option<(MatchEvent, fn(&Self, &mut Event<Cfg>))>; 32]>,
  resps: RefCell<ArrayVec<[Option<Resp<Cfg>>; 64]>>,
}

impl<Sock: Socket, Cfg: Config> Core<Sock, Cfg> {
  /// Creates a new Core with the default runtime behavior
  pub fn new() -> Self {
    let mut me = Self::behaviorless();
    me.bootstrap();
    me
  }

  /// Create a new runtime without any actual behavior
  ///
  /// ```
  /// use kwap::{config::Alloc, core::Core, std::UdpSocket};
  ///
  /// Core::<UdpSocket, Alloc>::behaviorless();
  /// ```
  pub fn behaviorless() -> Self {
    Self { resps: Default::default(),
           sock: Sock::default(),
           ears: Default::default() }
  }

  /// Add the default behavior to a behaviorless Core
  pub fn bootstrap(&mut self) {
    self.listen(MatchEvent::RecvDgram, try_parse_message);
    self.listen(MatchEvent::RecvMsg, resp_from_msg);
    self.listen(MatchEvent::RecvResp, Core::<Sock, Cfg>::store_resp);
  }

  /// Listens for RecvResp events and stores them on the runtime struct
  pub fn store_resp(&self, ev: &mut Event<Cfg>) {
    let resp = ev.get_mut_resp().unwrap().take().unwrap();
    let mut resps = self.resps.borrow_mut();
    if let Some(resp) = resps.try_push(Some(resp)) {
      // arrayvec is full, remove nones
      *resps = resps.iter_mut().filter_map(|o| o.take()).map(Some).collect();

      // panic if we're still full
      resps.push(resp);
    }
  }

  /// Listen for an event
  ///
  /// For an example, see [`Core.fire()`](#method.fire)
  pub fn listen(&mut self, mat: MatchEvent, listener: fn(&Self, &mut Event<Cfg>)) {
    self.ears.push(Some((mat, listener)));
  }

  /// Fire an event
  ///
  /// ```
  /// use kwap::{config::Alloc,
  ///            core::{event::{Event, MatchEvent},
  ///                   Core},
  ///            std::UdpSocket};
  /// use kwap_msg::MessageParseError::UnexpectedEndOfStream;
  ///
  /// static mut LOG_ERRS_CALLS: u8 = 0;
  ///
  /// fn log_errs(_: &Core<UdpSocket, Alloc>, ev: &mut Event<Alloc>) {
  ///   let err = ev.get_msg_parse_error().unwrap();
  ///   eprintln!("error! {:?}", err);
  ///   unsafe {
  ///     LOG_ERRS_CALLS += 1;
  ///   }
  /// }
  ///
  /// let mut client = Core::behaviorless();
  ///
  /// client.listen(MatchEvent::MsgParseError, log_errs);
  /// client.fire(Event::<Alloc>::MsgParseError(UnexpectedEndOfStream));
  ///
  /// unsafe { assert_eq!(LOG_ERRS_CALLS, 1) }
  /// ```
  pub fn fire(&self, event: Event<Cfg>) {
    let mut sound = event;
    self.ears.iter().filter_map(|o| o.as_ref()).for_each(|(mat, work)| {
                                                 if mat.matches(&sound) {
                                                   work(self, &mut sound);
                                                 }
                                               });
  }

  /// Check the stored socket for a new datagram, and fire a RecvDgram event
  fn poll_sock(&mut self) {
    let mut buf = ArrayVec::<[u8; 1152]>::new();
    let recvd = self.sock.recv(&mut buf);
    match recvd {
      | Ok(_) => {
        let ev = Event::RecvDgram(Some(buf));
        self.fire(ev);
      },
      // TODO: handle wouldblock and errors separately
      | _ => {},
    }
  }

  /// Poll for a response to a sent request
  pub fn poll_resp(&mut self, req_id: kwap_msg::Id) -> Result<Option<Resp<Cfg>>, ()> {
    self.poll_sock();
    let mut resps = self.resps.borrow_mut();
    let id_matches = |o: &Option<Resp<Cfg>>| o.as_ref().unwrap().msg.id == req_id;
    let resp = resps.iter_mut().find_map(|rep| match rep {
                                 | mut o @ Some(_) if id_matches(&o) => Option::take(&mut o),
                                 | _ => None,
                               });

    Ok(resp)
  }

  /// Send a message
  pub fn send_req(&mut self, req: Req<Cfg>) -> Result<(), ()> {
    let msg = config::Message::<Cfg>::from(req);
    let (_, host) = msg.opts
                       .iter()
                       .enumerate_option_numbers()
                       .find(|(n, _)| n.0 == 3)
                       .unwrap();
    let host_str: &str = core::str::from_utf8(&host.value.0).unwrap();
    self.sock
        .connect(SocketAddrV4::from_str(host_str).unwrap())
        .map_err(|_| ())?;
    self.sock
        .send(&msg.try_into_bytes::<ArrayVec<[u8; 1152]>>().map_err(|_| ())?)
        .map_err(|_| ())?;
    Ok(())
  }
}

impl<Sock: Socket, Cfg: Config> Eventer<Cfg> for Core<Sock, Cfg> {
  fn fire(&self, ev: Event<Cfg>) {
    self.fire(ev)
  }

  fn listen(&mut self, mat: MatchEvent, f: fn(&Self, &mut Event<Cfg>)) {
    self.listen(mat, f)
  }
}

#[cfg(test)]
mod tests {
  use kwap_msg::TryIntoBytes;
  use tinyvec::ArrayVec;

  use super::*;
  use crate::{config, config::Alloc, req::Req, test::TubeSock};

  #[test]
  fn eventer() {
    let req = Req::<Alloc>::get("0.0.0.0", 1234, "");
    let bytes = config::Message::<Alloc>::from(req).try_into_bytes::<ArrayVec<[u8; 1152]>>()
                                                   .unwrap();
    let mut client = Core::<TubeSock, Alloc>::behaviorless();

    fn on_err(_: &Core<TubeSock, Alloc>, e: &mut Event<Alloc>) {
      panic!("{:?}", e)
    }

    static mut CALLS: usize = 0;
    fn on_dgram(_: &Core<TubeSock, Alloc>, _: &mut Event<Alloc>) {
      unsafe {
        CALLS += 1;
      }
    }

    client.listen(MatchEvent::MsgParseError, on_err);
    client.listen(MatchEvent::RecvDgram, on_dgram);

    client.fire(Event::RecvDgram(Some(bytes)));

    unsafe {
      assert_eq!(CALLS, 1);
    }
  }

  #[test]
  fn client_flow() {
    type Msg = config::Message<Alloc>;

    let req = Req::<Alloc>::get("0.0.0.0", 1234, "");
    let id = req.msg.id;
    let resp = Resp::<Alloc>::for_request(req);
    let bytes = Msg::from(resp).try_into_bytes::<ArrayVec<[u8; 1152]>>().unwrap();

    let mut client = Core::<TubeSock, Alloc>::new();
    client.fire(Event::RecvDgram(Some(bytes)));

    let rep = client.poll_resp(id).unwrap().unwrap();
    assert_eq!(bytes, Msg::from(rep).try_into_bytes::<ArrayVec<[u8; 1152]>>().unwrap());
  }
}
