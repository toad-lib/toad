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
