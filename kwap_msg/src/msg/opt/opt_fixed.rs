  use super::*;

#[doc = opt_docs!()]
#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub struct Opt<const CAP: usize> {
  /// See [`Delta`]
  pub delta: OptDelta,
  /// See [`Value`]
  pub value: OptValue<CAP>,
}

  #[doc = value_docs!()]
  #[derive(Clone, PartialEq, PartialOrd, Debug)]
  pub struct OptValue<const CAP: usize>(pub ArrayVec<u8, CAP>);
