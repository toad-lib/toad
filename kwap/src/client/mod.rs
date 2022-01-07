use kwap_common::Insert;

use crate::{config::Config,
            event::{listeners::try_parse_message, Event, Eventer, MatchEvent}};

/// A CoAP request client that uses a state machine to send requests and process incoming messages.
///
/// The behavior at runtime is fully customizable, with the default provided via a [`Default::default`]
/// implementation.
#[allow(missing_debug_implementations)]
pub struct Client<Cfg: Config> {
  ears: Cfg::ClientEventHandlers,
}

impl<Cfg: Config> Default for Client<Cfg> {
  fn default() -> Self {
    let mut me = Self::behaviorless();
    me.bootstrap();
    me
  }
}

impl<Cfg: Config> Client<Cfg> {
  /// Create a new client without any actual behavior
  ///
  /// ```
  /// use kwap::{client::Client, config::Alloc};
  ///
  /// Client::<Alloc>::behaviorless();
  /// ```
  pub fn behaviorless() -> Self {
    Self { ears: Default::default() }
  }

  /// Add the default behavior to a behaviorless Client
  pub fn bootstrap(&mut self) {
    self.listen(MatchEvent::RecvDgram, try_parse_message);
  }

  /// Listen for an event
  ///
  /// For an example, see [`Client.fire()`](#method.fire)
  pub fn listen(&mut self, mat: MatchEvent, listener: fn(&Self, &mut Event<Cfg>)) {
    self.ears.push((mat, listener));
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
    self.ears.iter().for_each(|(mat, work)| {
                      if mat.matches(&sound) {
                        work(self, &mut sound);
                      }
                    });
  }
}

impl<Cfg: Config> Eventer<Cfg> for Client<Cfg> {
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
  use crate::{config, config::Alloc, req::Req};

  #[test]
  fn events_simple() {
    let req = Req::<Alloc>::get("foo");
    let bytes = config::Message::<Alloc>::from(req).try_into_bytes::<ArrayVec<[u8; 1152]>>()
                                                   .unwrap();
    let mut client = Client::<Alloc>::default();

    fn on_err(_: &Client<Alloc>, e: &mut Event<Alloc>) {
      panic!("{:?}", e)
    }

    static mut CALLS: usize = 0;
    fn on_msg(_: &Client<Alloc>, _: &mut Event<Alloc>) {
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
