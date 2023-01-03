#[derive(Debug, Clone, Copy)]
pub enum Unhydrated {}

#[derive(Debug, Clone, Copy)]
pub enum CompleteWhenHydrated {}

#[derive(Debug, Clone, Copy)]
pub enum Hydrated {}

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

pub trait Combine<T> {
  type Out: ApState;
}

pub trait ApState:
  Combine<super::Unhydrated>
  + Combine<super::Hydrated>
  + Combine<super::CompleteWhenHydrated>
  + Combine<super::Complete>
{
}
