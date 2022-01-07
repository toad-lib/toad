use no_std_net::SocketAddrV4;
use core::str::FromStr;

use kwap_common::Insert;
use kwap_msg::EnumerateOptNumbers;
use tinyvec::ArrayVec;

use crate::{config::{Config, self},
            event::{listeners::try_parse_message, Event, Eventer, MatchEvent}, resp::Resp, req::Req, Socket};

/// A CoAP request client that uses a state machine to send requests and process incoming messages.
///
/// The behavior at runtime is fully customizable, with the default provided via a [`Default::default`]
/// implementation.
#[allow(missing_debug_implementations)]
pub struct Client<Sock: Socket, Cfg: Config> {
  sock: Sock,
  ears: ArrayVec<[Option<(MatchEvent, fn(&Self, &mut Event<Cfg>))>; 32]>,
}

impl<Sock: Socket, Cfg: Config> Client<Sock, Cfg> {
  /// TODO
  pub fn new() -> Self {
    let mut me = Self::behaviorless();
    me.bootstrap();
    me
  }
  /// Create a new client without any actual behavior
  ///
  /// ```
  /// use kwap::{client::Client, config::Alloc};
  ///
  /// Client::<Alloc>::behaviorless();
  /// ```
  pub fn behaviorless() -> Self {
    Self { sock: Sock::default(), ears: Default::default() }
  }

  /// Add the default behavior to a behaviorless Client
  pub fn bootstrap(&mut self) {
    self.listen(MatchEvent::RecvDgram, try_parse_message);
  }

  /// Listen for an event
  ///
  /// For an example, see [`Client.fire()`](#method.fire)
  pub fn listen(&mut self, mat: MatchEvent, listener: fn(&Self, &mut Event<Cfg>)) {
    self.ears.push(Some((mat, listener)));
  }

  /// Fire an event
  ///
  /// ```
  /// use kwap::{client::Client,
  ///            config::Alloc,
  ///            event::{Event, MatchEvent}};
  /// use kwap_msg::MessageParseError::UnexpectedEndOfStream;
  ///
  /// static mut LOG_ERRS_CALLS: u8 = 0;
  ///
  /// fn log_errs(_: &Client<Alloc>, ev: &mut Event<Alloc>) {
  ///   let err = ev.get_msg_parse_error().unwrap();
  ///   eprintln!("error! {:?}", err);
  ///   unsafe {
  ///     LOG_ERRS_CALLS += 1;
  ///   }
  /// }
  ///
  /// let mut client = Client::behaviorless();
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

  /// Send a request!
  pub fn send(&mut self, msg: config::Message<Cfg>) -> Result<Resp<Cfg>, ()> {
    // connect to UDP socket
    let (_, host) = msg.opts.iter().enumerate_option_numbers().find(|(n, _)| n.0 == 3).unwrap();
    let host_str: &str = core::str::from_utf8(&host.value.0).unwrap();
    self.sock.connect(SocketAddrV4::from_str(host_str).unwrap());
    // send message
    // spinlock until a handler places a response in the response map
    Err(())
  }
}

impl<Sock: Socket, Cfg: Config> Eventer<Cfg> for Client<Sock, Cfg> {
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
  use crate::{test::TubeSock, config, config::Alloc, req::Req};

  #[test]
  fn events_simple() {
    let req = Req::<Alloc>::get("0.0.0.0", 1234, "");
    let bytes = config::Message::<Alloc>::from(req).try_into_bytes::<ArrayVec<[u8; 1152]>>()
                                                   .unwrap();
    let mut client = Client::<TubeSock, Alloc>::new();

    fn on_err(_: &Client<TubeSock, Alloc>, e: &mut Event<Alloc>) {
      panic!("{:?}", e)
    }

    static mut CALLS: usize = 0;
    fn on_msg(_: &Client<TubeSock, Alloc>, _: &mut Event<Alloc>) {
      println!("bar");
      unsafe {
        CALLS += 1;
      }
    }

    client.listen(MatchEvent::MsgParseError, on_err);
    client.listen(MatchEvent::RecvMsg, on_msg);

    client.fire(Event::RecvDgram(Some(bytes)));

    unsafe {
      assert_eq!(CALLS, 1);
    }
  }
}
