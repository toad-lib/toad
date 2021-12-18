use super::*;

#[doc = msg_docs!()]
#[cfg_attr(any(feature = "docs", docsrs), doc(cfg(feature = "alloc")))]
#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub struct Message<const N_OPTS: usize, const OPT_CAP: usize> {
  /// see [`Id`] for details
  pub id: Id,
  /// see [`Type`] for details
  pub ty: Type,
  /// see [`Version`] for details
  pub ver: Version,
  /// see [`TokenLength`] for details
  pub tkl: TokenLength,
  /// see [`Token`] for details
  pub token: Token,
  /// see [`Code`] for details
  pub code: Code,
  /// see [`opt::Opt`] for details
  pub opts: ArrayVec<opt_fixed::Opt<OPT_CAP>, N_OPTS>,
  /// See [`Payload`]
  pub payload: Payload,
}
