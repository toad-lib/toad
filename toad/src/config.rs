use embedded_time::duration::Milliseconds;

use crate::retry::{Attempts, Strategy};
use crate::time::Millis;

/// Bytes / Second
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BytesPerSecond(pub u16);

/// Configuration options related to parsing & handling outbound CON requests
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Con {
  /// Retry strategy for CON requests that
  /// have not yet been ACKed.
  ///
  /// Defaults to an exponential retry strategy:
  /// ```
  /// use embedded_time::duration::Milliseconds;
  /// use toad::config::Con;
  /// use toad::retry::Strategy;
  ///
  /// assert_eq!(Con::default().unacked_retry_strategy,
  ///            Strategy::Exponential { init_min: Milliseconds(500),
  ///                                    init_max: Milliseconds(1_000) });
  /// ```
  pub unacked_retry_strategy: Strategy,
  /// Retry strategy for CON requests that have been ACKed.
  ///
  /// Usually this should be **lazier** than `unacked_retry_strategy`,
  /// since we can reasonably expect the duration between "received request"
  /// and "responded with ACK" to be much shorter than "responded with ACK" and
  /// "sent actual response."
  ///
  /// Defaults to a lazy exponential retry strategy:
  /// ```
  /// use embedded_time::duration::Milliseconds;
  /// use toad::config::Con;
  /// use toad::retry::Strategy;
  ///
  /// assert_eq!(Con::default().acked_retry_strategy,
  ///            Strategy::Exponential { init_min: Milliseconds(1_000),
  ///                                    init_max: Milliseconds(2_000) });
  /// ```
  pub acked_retry_strategy: Strategy,
  /// Number of times we are allowed to resend a CON request
  /// before erroring.
  //
  /// Defaults to 4 attempts.
  /// ```
  /// use toad::config::Con;
  /// use toad::retry::Attempts;
  ///
  /// assert_eq!(Con::default().max_attempts, Attempts(4));
  /// ```
  pub max_attempts: Attempts,
}

/// Configuration options related to parsing & handling outbound NON requests
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Non {
  /// Strategy to use when we sent a NON request and haven't yet
  /// received a response.
  ///
  /// **Note** that in a future commit there will be a method by which NON
  /// requests can be "flung" without any expectation of a response.
  ///
  /// Defaults to a pessimistic exponential retry strategy:
  /// ```
  /// use embedded_time::duration::Milliseconds;
  /// use toad::config::Non;
  /// use toad::retry::Strategy;
  ///
  /// assert_eq!(Non::default().retry_strategy,
  ///            Strategy::Exponential { init_min: Milliseconds(250),
  ///                                    init_max: Milliseconds(500) });
  /// ```
  pub retry_strategy: Strategy,
  /// Number of times we are allowed to resend a NON request
  /// before erroring.
  ///
  /// Defaults to 4 attempts.
  /// ```
  /// use toad::config::Non;
  /// use toad::retry::Attempts;
  ///
  /// assert_eq!(Non::default().max_attempts, Attempts(4));
  /// ```
  pub max_attempts: Attempts,
}

/// Configuration options related to parsing & handling messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Msg {
  /// Seed used to generate message [`Token`](toad_msg::Token)s,
  /// customizable to allow for your application to generate tokens
  /// less guessably.
  ///
  /// The default value is 0, although it is
  /// best practice to set this to something else.
  /// (random integer, machine identifier)
  ///
  /// _e.g. if you're developing a swarm of
  /// smart CoAP-enabled thermostats, each one would ideally
  /// have a distinct token seed._
  ///
  /// ```
  /// use toad::config::Msg;
  ///
  /// assert_eq!(Msg::default().token_seed, 0);
  /// ```
  // token_seed
  // ||
  // xx xxxxxxxx
  //    |      |
  //    timestamp
  pub token_seed: u16,

  /// Set the transmission rate that we should do our best
  /// not to exceed when waiting for:
  /// - responses to our NON requests
  /// - responses to our acked CON requests
  ///
  /// Defaults to `BytesPerSecond(1000)`
  ///
  /// ```
  /// use toad::config::{BytesPerSecond, Msg};
  ///
  /// assert_eq!(Msg::default().probing_rate, BytesPerSecond(1000));
  /// ```
  pub probing_rate: BytesPerSecond,

  /// See [`Con`]
  pub con: Con,

  /// See [`Non`]
  pub non: Non,

  /// Set the maximum amount of time we should delay
  /// our response to multicast requests.
  ///
  /// The actual delay will be random between zero
  /// and this value.
  ///
  /// Defaults to 5000 milliseconds.
  ///
  /// ```
  /// use embedded_time::duration::Milliseconds;
  /// use toad::config::Msg;
  ///
  /// assert_eq!(Msg::default().multicast_response_leisure,
  ///            Milliseconds(5000u64));
  /// ```
  pub multicast_response_leisure: Millis,
}

impl Default for Con {
  fn default() -> Self {
    Con { unacked_retry_strategy: Strategy::Exponential { init_min: Milliseconds(500),
                                                          init_max: Milliseconds(1_000) },
          acked_retry_strategy: Strategy::Exponential { init_min: Milliseconds(1_000),
                                                        init_max: Milliseconds(2_000) },
          max_attempts: Attempts(4) }
  }
}

impl Default for Non {
  fn default() -> Self {
    Non { retry_strategy: Strategy::Exponential { init_min: Milliseconds(250),
                                                  init_max: Milliseconds(500) },
          max_attempts: Attempts(4) }
  }
}

impl Default for Msg {
  fn default() -> Self {
    Msg { token_seed: 0,
          probing_rate: BytesPerSecond(1000),
          con: Con::default(),
          non: Non::default(),
          multicast_response_leisure: Milliseconds(5000) }
  }
}

/// Runtime config
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Config {
  /// See [`Msg`]
  pub msg: Msg,
  /// Maximum number of requests that
  /// can be in flight at a given moment
  ///
  /// Default value is `1` (no concurrency)
  ///
  /// ```
  /// use toad::config::Config;
  ///
  /// assert_eq!(Config::default().max_concurrent_requests, 1);
  /// ```
  pub max_concurrent_requests: u8,
}

impl Default for Config {
  fn default() -> Self {
    Config { msg: Msg::default(),
             max_concurrent_requests: 1 }
  }
}

impl Config {
  pub(crate) fn max_transmit_span_millis(&self) -> u64 {
    let acked_con = self.msg
                        .con
                        .acked_retry_strategy
                        .max_time(self.msg.con.max_attempts - Attempts(1))
                        .0 as u64;

    let unacked_con = self.msg
                          .con
                          .unacked_retry_strategy
                          .max_time(self.msg.con.max_attempts - Attempts(1))
                          .0 as u64;

    let non = self.msg
                  .non
                  .retry_strategy
                  .max_time(self.msg.non.max_attempts - Attempts(1))
                  .0 as u64;

    acked_con.max(unacked_con).max(non)
  }

  pub(crate) fn max_transmit_wait_millis(&self) -> u64 {
    let acked_con = self.msg
                        .con
                        .acked_retry_strategy
                        .max_time(self.msg.con.max_attempts)
                        .0 as u64;

    let unacked_con = self.msg
                          .con
                          .unacked_retry_strategy
                          .max_time(self.msg.con.max_attempts)
                          .0 as u64;

    let non = self.msg
                  .non
                  .retry_strategy
                  .max_time(self.msg.non.max_attempts)
                  .0 as u64;

    acked_con.max(unacked_con).max(non)
  }

  // TODO: adjust these on the fly based on actual timings?
  pub(crate) fn max_latency_millis(&self) -> u64 {
    100_000
  }

  pub(crate) fn expected_processing_delay_millis(&self) -> u64 {
    200
  }

  pub(crate) fn exchange_lifetime_millis(&self) -> u64 {
    self.max_transmit_span_millis()
    + (2 * self.max_latency_millis())
    + self.expected_processing_delay_millis()
  }
}
