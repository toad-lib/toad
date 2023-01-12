#[allow(unused_imports)]
use super::Ap;

/// [`Ap::ok`]
#[derive(Debug, Clone, Copy)]
pub enum Unhydrated {}

/// [`Ap::reject`], [`Ap::respond`]
#[derive(Debug, Clone, Copy)]
pub enum CompleteWhenHydrated {}

/// [`Ap::ok_hydrated`]
#[derive(Debug, Clone, Copy)]
pub enum Hydrated {}

/// [`Ap::reject_hydrated`], [`Ap::respond_hydrated`], [`Ap::err`]
#[derive(Debug, Clone, Copy)]
pub enum Complete {}

impl Combine<Unhydrated> for Unhydrated {
  type Out = Unhydrated;
}

impl Combine<Hydrated> for Unhydrated {
  type Out = Hydrated;
}

impl Combine<CompleteWhenHydrated> for Unhydrated {
  type Out = CompleteWhenHydrated;
}

impl Combine<Complete> for Unhydrated {
  type Out = Complete;
}

impl Combine<Unhydrated> for Hydrated {
  type Out = Hydrated;
}

impl Combine<Hydrated> for Hydrated {
  type Out = Hydrated;
}

impl Combine<CompleteWhenHydrated> for Hydrated {
  type Out = Complete;
}

impl Combine<Complete> for Hydrated {
  type Out = Complete;
}

impl Combine<Unhydrated> for Complete {
  type Out = Complete;
}

impl Combine<Hydrated> for Complete {
  type Out = Complete;
}

impl Combine<CompleteWhenHydrated> for Complete {
  type Out = Complete;
}

impl Combine<Complete> for Complete {
  type Out = Complete;
}

impl Combine<Unhydrated> for CompleteWhenHydrated {
  type Out = CompleteWhenHydrated;
}

impl Combine<Hydrated> for CompleteWhenHydrated {
  type Out = Complete;
}

impl Combine<CompleteWhenHydrated> for CompleteWhenHydrated {
  type Out = CompleteWhenHydrated;
}

impl Combine<Complete> for CompleteWhenHydrated {
  type Out = Complete;
}

impl ApState for Hydrated {}
impl ApState for Unhydrated {}
impl ApState for Complete {}
impl ApState for CompleteWhenHydrated {}

/// What happens when an `Ap` with
/// `ApState` of `Self` is combined with `T`?
///
/// e.g. Combining `CompleteWhenHydrated` with `Hydrated` is `Complete`.
///
/// `<CompleteWhenHydrated as Combine<Hydrated>>::Out` is `Complete`
pub trait Combine<T> {
  /// Result of combination
  type Out: ApState;
}

/// Completeness of an Ap.
///
/// For more context see [`Ap`].
pub trait ApState:
  Combine<super::Unhydrated>
  + Combine<super::Hydrated>
  + Combine<super::CompleteWhenHydrated>
  + Combine<super::Complete>
{
}
