//! `kwap` is a Rust CoAP implementation that aims to be:
//! - Platform-independent
//! - Extensible
//! - Approachable
//!
//! ## CoAP
//! CoAP is an application-level network protocol that copies the semantics of HTTP
//! to an environment conducive to **constrained** devices. (weak hardware, small battery capacity, etc.)
//!
//! This means that you can write and run two-way RESTful communication
//! between devices very similarly to the networking semantics you are
//! most likely very familiar with.
//!
//! ### Similarities to HTTP
//! CoAP has the same verbs and many of the same semantics as HTTP;
//! - GET, POST, PUT, DELETE
//! - Headers (renamed to [Options](https://datatracker.ietf.org/doc/html/rfc7252#section-5.10))
//! - Data format independent (via the [Content-Format](https://datatracker.ietf.org/doc/html/rfc7252#section-12.3) Option)
//! - [Response status codes](https://datatracker.ietf.org/doc/html/rfc7252#section-5.9)
//!
//! ### Differences from HTTP
//! - CoAP customarily sits on top of UDP (however the standard is [in the process of being adapted](https://tools.ietf.org/id/draft-ietf-core-coap-tcp-tls-11.html) to also run on TCP, like HTTP)
//! - Because UDP is a "connectionless" protocol, it offers no guarantee of "conversation" between traditional client and server roles. All the UDP transport layer gives you is a method to listen for messages thrown at you, and to throw messages at someone. Owing to this, CoAP machines are expected to perform both client and server roles (or more accurately, _sender_ and _receiver_ roles)
//! - While _classes_ of status codes are the same (Success 2xx -> 2.xx, Client error 4xx -> 4.xx, Server error 5xx -> 5.xx), the semantics of the individual response codes differ.

#![doc(html_root_url = "https://docs.rs/kwap/0.1.2")]
#![cfg_attr(all(not(test), feature = "no_std"), no_std)]
#![cfg_attr(not(test),
            deny(missing_debug_implementations,
                 unreachable_pub,
                 unsafe_code,
                 missing_copy_implementations))]
#![cfg_attr(any(docsrs, feature = "docs"), feature(doc_cfg))]
#![deny(missing_docs)]

#[cfg(feature = "alloc")]
extern crate alloc as std_alloc;

/// CoAP response messages
pub mod resp;

/// CoAP request messages
pub mod req;

/// CoAP client
pub mod event;

/// CoAP client
pub mod client;

/// kwap configuration
pub mod config;

static mut ID: u16 = 0;

fn generate_id() -> kwap_msg::Id {
  // TEMPORARY
  // TODO: replace with long-living Client or Endpoint structure
  #[allow(unsafe_code)]
  unsafe {
    ID += 1;
    kwap_msg::Id(ID)
  }
}

fn add_option<A: Array<Item = (OptNumber, Opt<B>)>, B: Array<Item = u8>, V: IntoIterator<Item = u8>>(
  opts: &mut A,
  number: u32,
  value: V)
  -> Option<(u32, V)> {
  use kwap_msg::*;

  let exist = opts.iter_mut().find(|(OptNumber(num), _)| *num == number);

  if let Some((_, opt)) = exist {
    opt.value = OptValue(value.into_iter().collect());
    return None;
  }

  let n_opts = opts.get_size() + 1;
  let no_room = opts.max_size().map(|max| max < n_opts).unwrap_or(false);

  if no_room {
    return Some((number, value));
  }

  let num = OptNumber(number);
  let opt = Opt::<_> { delta: Default::default(),
                       value: OptValue(value.into_iter().collect()) };

  opts.extend(Some((num, opt)));

  None
}

fn normalize_opts<OptNumbers: Array<Item = (OptNumber, Opt<Bytes>)>,
                  Opts: Array<Item = Opt<Bytes>>,
                  Bytes: Array<Item = u8>>(
  mut os: OptNumbers)
  -> Opts {
  if os.is_empty() {
    return Opts::default();
  }

  os.sort_by_key(|&(OptNumber(num), _)| num);
  os.into_iter().fold(Opts::default(), |mut opts, (num, mut opt)| {
                  let delta = opts.last().map(|Opt { delta: OptDelta(del), .. }| *del).unwrap_or(0u16);
                  opt.delta = OptDelta((num.0 as u16) - delta);
                  opts.push(opt);
                  opts
                })
}

macro_rules! code {
  (rfc7252($section:literal) $name:ident = $c:literal.$d:literal) => {
    #[doc = kwap_macros::rfc_7252_doc!($section)]
    #[allow(clippy::zero_prefixed_literal)]
    pub const $name: kwap_msg::Code = kwap_msg::Code::new($c, $d);
  };
  (rfc7252($section:literal) $name:ident = $newtype:tt($c:literal.$d:literal)) => {
    #[doc = kwap_macros::rfc_7252_doc!($section)]
    #[allow(clippy::zero_prefixed_literal)]
    pub const $name: $newtype = $newtype(kwap_msg::Code::new($c, $d));
  };
}

pub(crate) use code;
use kwap_common::Array;
use kwap_msg::{Opt, OptDelta, OptNumber};
