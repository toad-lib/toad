/// Version of the CoAP protocol that the message adheres to.
///
/// Right now, this will always be 1, but may support additional values in the future.
///
/// See [RFC7252 - Message Details](https://datatracker.ietf.org/doc/html/rfc7252#section-3) for context
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct Version(pub u8);

impl Default for Version {
  fn default() -> Self {
    Version(1)
  }
}
