use core::cell::Cell;
use kwap_common::{Array, Insert};

use std_alloc::vec::Vec;

use crate::{req::Req, event::{Event, MatchEvent}, config::Config};

/// A CoAP Endpoint (client or server)
#[allow(missing_debug_implementations)]
pub struct Endpoint<T, A: Array<Item = (MatchEvent, fn(&T, &Event<Cfg>))>, Cfg: Config> {
  ears: Cell<A>,
}

impl<T, A: Array<Item = (MatchEvent, fn(&T, &Event<Cfg>))>, Cfg: Config> Default for Endpoint<T, A, Cfg> {
  fn default() -> Self {
    Self::new()
  }
}

impl<T, A: Array<Item = (MatchEvent, fn(&T, &Event<Cfg>))>, Cfg: Config> Endpoint<T, A, Cfg> {
  /// Create a new endpoint
  ///
  /// ```
  /// use kwap::{config::Alloc, client::Endpoint};
  ///
  /// let ep = Endpoint::<Alloc>::new();
  /// ```
  pub fn new() -> Self {
    Self {
      ears: Default::default(),
    }
  }

  /// Listen for an event
  ///
  /// ```
  /// use kwap::config::Alloc;
  /// use kwap::client::Endpoint;
  /// use kwap::event::{Event, MatchEvent};
  ///
  /// fn log_errs(_: &Endpoint<Alloc>, ev: &Event<Alloc>) {
  ///   let err = ev.get_msg_parse_error().unwrap();
  ///   eprintln!("error! {:?}", err);
  /// }
  ///
  /// let ep = Endpoint::new();
  ///
  /// ep.attach(MatchEvent::MsgParseError, log_errs);
  /// ```
  pub fn attach(&self, mat: MatchEvent, listener: fn(&T, &Event<Cfg>)) {
    let mut ears = self.ears.take();
    ears.push((mat, listener));
    self.ears.set(ears);
  }

  /// Fire an event
  ///
  /// ```
  /// use kwap_msg::MessageParseError::UnexpectedEndOfStream;
  /// use kwap::config::Alloc;
  /// use kwap::client::Endpoint;
  /// use kwap::event::{Event, MatchEvent};
  ///
  /// static mut LOG_ERRS_WAS_CALLED: bool = false;
  ///
  /// fn log_errs(_: &Endpoint<Alloc>, ev: &Event<Alloc>) {
  ///   let err = ev.get_msg_parse_error().unwrap();
  ///   eprintln!("error! {:?}", err);
  ///   unsafe {LOG_ERRS_WAS_CALLED = true;}
  /// }
  ///
  /// let ep = Endpoint::new();
  ///
  /// ep.attach(MatchEvent::MsgParseError, log_errs);
  ///
  /// ep.fire(Event::<Alloc>::MsgParseError(UnexpectedEndOfStream));
  ///
  /// unsafe {assert!(LOG_ERRS_WAS_CALLED)}
  /// ```
  pub fn fire(&self, event: Event<Cfg>) {
    let sound = event;
    let ears = Cell::take(&self.ears);
    ears.iter()
        .filter(|(mat, _)| mat.matches(&sound))
        .for_each(|(_, work)| work(self, &sound));
    self.ears.set(ears)
  }
}
