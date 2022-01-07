use core::fmt::Debug;

use kwap_common::Array;
use kwap_msg::{Opt, OptNumber};
#[cfg(feature = "alloc")]
use std_alloc::vec::Vec;

use crate::{client::Client,
            event::{Event, MatchEvent}};

/// Configures `kwap` to use `Vec` for collections
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, Copy)]
pub struct Alloc;

#[cfg(feature = "alloc")]
impl Config for Alloc {
  type PayloadBuffer = Vec<u8>;
  type OptBytes = Vec<u8>;
  type Opts = Vec<Opt<Vec<u8>>>;
  type OptNumbers = Vec<(OptNumber, Opt<Vec<u8>>)>;
  type Events = Vec<Event<Self>>;
  type ClientEventHandlers = Vec<(MatchEvent, fn(&Client<Self>, &mut Event<Self>))>;
}

/// kwap configuration trait
pub trait Config: Sized + 'static {
  /// What type should we use to store the message payloads?
  type PayloadBuffer: Array<Item = u8> + Clone + Debug;
  /// What type should we use to store the option values?
  type OptBytes: Array<Item = u8> + 'static + Clone + Debug;
  /// What type should we use to store the options?
  type Opts: Array<Item = Opt<Self::OptBytes>> + Clone + Debug;

  /// What type should we use to keep track of options before serializing?
  type OptNumbers: Array<Item = (OptNumber, Opt<Self::OptBytes>)> + Clone + Debug;

  /// What type should we use to store event handlers?
  type ClientEventHandlers: Array<Item = (MatchEvent, fn(&Client<Self>, &mut Event<Self>))>;

  /// What type should we use to store events?
  type Events: Array<Item = Event<Self>>;
}

pub(crate) type Message<Cfg> =
  kwap_msg::Message<<Cfg as Config>::PayloadBuffer, <Cfg as Config>::OptBytes, <Cfg as Config>::Opts>;
