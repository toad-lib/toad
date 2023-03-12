use toad_msg::Code;

use super::ap::state::CompleteWhenHydrated;
use super::ap::{Ap, Respond};
use crate::platform::PlatformTypes;

/// Respond to the incoming request, with a custom code and payload.
pub fn respond<P, E>(code: Code, payload: P::MessagePayload) -> Ap<CompleteWhenHydrated, P, (), E>
  where P: PlatformTypes,
        E: core::fmt::Debug
{
  Ap::respond(Respond { code,
                        payload,
                        etag: None })
}

/// [`respond`] with 2.05 CONTENT
pub fn ok<P, E>(payload: P::MessagePayload) -> Ap<CompleteWhenHydrated, P, (), E>
  where P: PlatformTypes,
        E: core::fmt::Debug
{
  respond(crate::resp::code::CONTENT, payload)
}

/// [`respond`] with 4.04 NOT FOUND
pub fn not_found<P, E>(payload: P::MessagePayload) -> Ap<CompleteWhenHydrated, P, (), E>
  where P: PlatformTypes,
        E: core::fmt::Debug
{
  respond(crate::resp::code::NOT_FOUND, payload)
}

/// Respond with JSON
#[cfg(any(feature = "std_serde_json", feature = "unstable_serde_json"))]
pub mod json {
  use serde::Serialize;
  #[cfg(feature = "std_serde_json")]
  use serde_json::Error;
  #[cfg(feature = "unstable_serde_json")]
  use serde_json_core::ser::Error;

  use super::*;

  /// Errors that can be constructed from [`serde_json::Error`]
  #[cfg(feature = "std_serde_json")]
  pub trait SerializeError: Sized + core::fmt::Debug {
    #[allow(missing_docs)]
    fn json_error(e: serde_json::Error) -> Self;
  }

  #[cfg(feature = "std_serde_json")]
  impl SerializeError for std::io::Error {
    fn json_error(e: serde_json::Error) -> Self {
      std::io::Error::new(std::io::ErrorKind::Other, e)
    }
  }

  /// Errors that can be constructed from [`serde_json_core::ser::Error`]
  #[cfg(feature = "unstable_serde_json")]
  pub trait SerializeError: Sized + core::fmt::Debug {
    #[allow(missing_docs)]
    fn json_error(e: serde_json_core::ser::Error) -> Self;
  }

  #[cfg(any(test, feature = "unstable_serde_json"))]
  fn ok_no_std<P, T>(t: T) -> Result<P::MessagePayload, serde_json_core::ser::Error>
    where P: PlatformTypes,
          T: Serialize
  {
    use toad_common::{Filled, Trunc};

    let mut p = P::MessagePayload::filled(0u8).expect("cannot combine dynamically allocated collections with no_std crate feature `unstable_serde_json`. Use `std_serde_json` instead.");
    serde_json_core::to_slice(&t, &mut p).map(|ct| {
                                           p.trunc(ct);
                                           p
                                         })
  }

  #[cfg(feature = "std_serde_json")]
  fn ok_std<P, T>(t: T) -> Result<P::MessagePayload, Error>
    where P: PlatformTypes,
          T: Serialize
  {
    serde_json::to_vec(&t).map(|v| v.into_iter().collect::<P::MessagePayload>())
  }

  /// Respond 2.05 CONTENT with a JSON payload of type `T`
  ///
  /// Supports `std` and non-`std` platforms.
  ///
  /// ```no_run
  /// use toad::std;
  /// use toad::step::runtime;
  /// use toad::config::Config;
  /// use toad::server::{Init, BlockingServer, path, respond};
  ///
  /// type Server = std::Platform<std::dtls::N, runtime::std::Runtime<std::dtls::N>>;
  ///
  /// enum Topping {
  ///   Pepperoni,
  ///   RedOnion,
  ///   Pineapple,
  /// }
  /// # impl Topping {
  /// #   fn to_json(&self) -> serde_json::Value {
  /// #     serde_json::Value::String(match self {
  /// #       Topping::Pepperoni => "Pepperoni",
  /// #       Topping::RedOnion => "RedOnion",
  /// #       Topping::Pineapple => "Pineapple",
  /// #     }.to_string())
  /// #   }
  /// # }
  /// # impl serde::Serialize for Topping {
  /// #   fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
  /// #     self.to_json().serialize(s)
  /// #   }
  /// # }
  ///
  /// struct Pizza {
  ///   toppings: Vec<Topping>,
  /// }
  /// # impl Pizza {
  /// #   fn to_json(&self) -> serde_json::Value {
  /// #     use serde_json::Value;
  /// #     let mut map = serde_json::Map::new();
  /// #     map.insert("toppings".to_string(), Value::Array(self.toppings.iter().map(|t| t.to_json()).collect::<Vec<_>>()));
  /// #     Value::Object(map)
  /// #   }
  /// # }
  /// # impl serde::Serialize for Pizza {
  /// #   fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
  /// #     self.to_json().serialize(s)
  /// #   }
  /// # }
  ///
  /// pub fn main() {
  ///   let server = Server::try_new("0.0.0.0:1111", Config::default()).unwrap();
  ///   server.run(Init::none(), |run| {
  ///     run.maybe(|ap| ap.pipe(path::check::rest_equals("pizza")).bind(|_| respond::json::ok(Pizza {toppings: vec![Topping::Pepperoni]})))
  ///   });
  /// }
  /// ```
  pub fn ok<P, T, E>(t: T) -> Ap<CompleteWhenHydrated, P, (), E>
    where E: SerializeError,
          P: PlatformTypes,
          T: Serialize
  {
    #[cfg(feature = "unstable_serde_json")]
    let ok = ok_no_std::<P, T>(t);
    #[cfg(feature = "std_serde_json")]
    let ok = ok_std::<P, T>(t);

    ok.map_err(E::json_error)
      .map(|p| super::ok(p))
      .unwrap_or_else(|e| Ap::err(e).pretend())
  }

  #[cfg(test)]
  #[allow(dead_code)]
  mod test {
    use serde::Serialize;
    use tinyvec::ArrayVec;
    use toad_msg::{OptNumber, OptValue};

    use crate::platform::{Effect, PlatformTypes};

    #[derive(Debug, Clone, Copy)]
    struct P;
    impl PlatformTypes for P {
      type MessagePayload = ArrayVec<[u8; 512]>;
      type MessageOptionBytes = ArrayVec<[u8; 128]>;
      type MessageOptionMapOptionValues = ArrayVec<[OptValue<Self::MessageOptionBytes>; 4]>;
      type MessageOptions = ArrayVec<[(OptNumber, Self::MessageOptionMapOptionValues); 4]>;
      type Clock = crate::test::ClockMock;
      type Socket = crate::test::SockMock;
      type Effects = ArrayVec<[Effect<Self>; 4]>;
    }

    #[derive(Debug, Clone, Serialize)]
    enum Topping {
      Pepperoni,
      Onion,
    }

    #[derive(Debug, Clone, Serialize)]
    struct Pizza(Vec<Topping>);

    #[derive(Debug, Clone, PartialEq, PartialOrd)]
    struct Error;

    impl super::SerializeError for Error {
      fn json_error(_: serde_json::Error) -> Self {
        Error
      }
    }

    #[test]
    fn ok_no_std_happy() {
      let pizza = Pizza(vec![Topping::Pepperoni]);
      let pizza_bytes = serde_json::to_vec(&pizza).unwrap()
                                                  .into_iter()
                                                  .collect::<ArrayVec<[u8; 512]>>();

      assert_eq!(super::ok_no_std::<P, Pizza>(pizza), Ok(pizza_bytes));
    }

    #[test]
    fn ok_std_happy() {
      let pizza = Pizza(vec![Topping::Pepperoni]);
      let pizza_bytes = serde_json::to_vec(&pizza).unwrap();

      assert_eq!(super::ok_std::<crate::std::PlatformTypes<crate::std::dtls::Y>, Pizza>(pizza).unwrap(), pizza_bytes);
    }
  }
}
