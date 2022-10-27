use super::MessageParseError;

/// Indicates if this message is of
/// type Confirmable (0), Non-confirmable (1), Acknowledgement (2), or Reset (3).
///
/// See [RFC7252 - Message Details](https://datatracker.ietf.org/doc/html/rfc7252#section-3) for context
#[derive(Copy, Clone, Hash, Eq, Ord, PartialEq, PartialOrd, Debug)]
pub enum Type {
  /// Some messages do not require an acknowledgement.  This is
  /// particularly true for messages that are repeated regularly for
  /// application requirements, such as repeated readings from a sensor.
  Non,
  /// Some messages require an acknowledgement.  These messages are
  /// called "Confirmable".  When no packets are lost, each Confirmable
  /// message elicits exactly one return message of type Acknowledgement
  /// or type Reset.
  Con,
  /// An Acknowledgement message acknowledges that a specific
  /// Confirmable message arrived.  By itself, an Acknowledgement
  /// message does not indicate success or failure of any request
  /// encapsulated in the Confirmable message, but the Acknowledgement
  /// message may also carry a Piggybacked Response.
  Ack,
  /// A Reset message indicates that a specific message (Confirmable or
  /// Non-confirmable) was received, but some context is missing to
  /// properly process it.  This condition is usually caused when the
  /// receiving node has rebooted and has forgotten some state that
  /// would be required to interpret the message.  Provoking a Reset
  /// message (e.g., by sending an Empty Confirmable message) is also
  /// useful as an inexpensive check of the liveness of an endpoint
  /// ("CoAP ping").
  Reset,
}

impl TryFrom<u8> for Type {
  type Error = MessageParseError;

  fn try_from(b: u8) -> Result<Self, Self::Error> {
    match b {
      | 0 => Ok(Type::Con),
      | 1 => Ok(Type::Non),
      | 2 => Ok(Type::Ack),
      | 3 => Ok(Type::Reset),
      | _ => Err(MessageParseError::InvalidType(b)),
    }
  }
}
