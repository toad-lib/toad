use toad_msg::Code;

use super::ap::state::CompleteWhenHydrated;
use super::ap::{Ap, Respond};
use crate::platform::PlatformTypes;

pub fn respond<P, E>(code: Code, payload: P::MessagePayload) -> Ap<CompleteWhenHydrated, P, (), E>
  where P: PlatformTypes,
        E: core::fmt::Debug
{
  Ap::respond(Respond { code,
                        payload,
                        etag: None })
}

pub fn ok<P, E>(payload: P::MessagePayload) -> Ap<CompleteWhenHydrated, P, (), E>
  where P: PlatformTypes,
        E: core::fmt::Debug
{
  respond(crate::resp::code::CONTENT, payload)
}
