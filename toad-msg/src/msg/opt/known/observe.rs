/// When included in a GET request, the Observe Option extends the GET
/// method so it does not only retrieve a current representation of the
/// target resource, but also requests the server to add or remove an
/// entry in the list of observers of the resource depending on the
/// option value.  The list entry consists of the client endpoint and the
/// token specified by the client in the request.  Possible values are:
///
///    `0` (register) adds the entry to the list, if not present;
///
///    `1` (deregister) removes the entry from the list, if present
#[derive(Hash, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Action {
  /// Tells the resource owner we would like to observe updates to
  /// the resource we've issued a GET request for.
  Register,
  /// Tells the resource owner we would no longer like to observe updates to
  /// the resource we've issued a GET request for.
  Deregister,
}

impl Action {
  /// Try to parse from a single byte
  pub fn from_byte(n: u8) -> Option<Self> {
    match n {
      | 0 => Some(Action::Register),
      | 1 => Some(Action::Deregister),
      | _ => None,
    }
  }
}

impl From<Action> for u8 {
  fn from(a: Action) -> Self {
    match a {
      | Action::Register => 0,
      | Action::Deregister => 1,
    }
  }
}
