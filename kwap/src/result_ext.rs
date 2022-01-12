pub(crate) trait ResultExt<T, E>: Sized {
  fn bind<R>(self, f: impl FnOnce(T) -> Result<R, E>) -> Result<R, E>;
  fn bind_err<R>(self, f: impl FnOnce(E) -> Result<T, R>) -> Result<T, R>;
  fn try_perform(self, f: impl FnOnce(&T) -> Result<(), E>) -> Result<T, E>;
  fn perform(self, f: impl FnOnce(&T) -> ()) -> Result<T, E>;
  fn filter(self, pred: impl FnOnce(&T) -> bool, on_fail: impl FnOnce(&T) -> E) -> Result<T, E>;
  fn tupled<R>(self, f: impl FnOnce(&T) -> Result<R, E>) -> Result<(T, R), E> {
    self.bind(|t| f(&t).map(|r| (t, r)))
  }
}

impl<T, E> ResultExt<T, E> for Result<T, E> {
  fn bind<R>(self, f: impl FnOnce(T) -> Result<R, E>) -> Result<R, E> {
    self.and_then(f)
  }

  fn bind_err<R>(self, f: impl FnOnce(E) -> Result<T, R>) -> Result<T, R> {
    match self {
      | Ok(t) => Ok(t),
      | Err(e) => f(e),
    }
  }

  fn try_perform(self, f: impl FnOnce(&T) -> Result<(), E>) -> Result<T, E> {
    self.and_then(|t| f(&t).map(|_| t))
  }

  fn perform(self, f: impl FnOnce(&T) -> ()) -> Result<T, E> {
    self.map(|t| {
          f(&t);
          t
        })
  }

  fn filter(self, pred: impl FnOnce(&T) -> bool, on_fail: impl FnOnce(&T) -> E) -> Result<T, E> {
    self.bind(|t| if pred(&t) { Err(on_fail(&t)) } else { Ok(t) })
  }
}
