pub(crate) trait ResultExt<T, E>: Sized {
  fn bind<R>(self, f: impl FnOnce(T) -> Result<R, E>) -> Result<R, E>;
  fn bind_err<R>(self, f: impl FnOnce(E) -> Result<T, R>) -> Result<T, R>;
  fn try_perform(self, f: impl FnOnce(&T) -> Result<(), E>) -> Result<T, E>;
  fn perform_err(self, f: impl FnOnce(&E) -> ()) -> Result<T, E>;
  fn perform(self, f: impl FnOnce(&T) -> ()) -> Result<T, E>;
  fn perform_mut(self, f: impl FnOnce(&mut T) -> ()) -> Result<T, E>;
  fn filter(self, pred: impl FnOnce(&T) -> bool, on_fail: impl FnOnce(&T) -> E) -> Result<T, E>;
  fn tupled<R>(self, f: impl FnOnce(&T) -> Result<R, E>) -> Result<(T, R), E> {
    self.bind(|t| f(&t).map(|r| (t, r)))
  }

  fn two<B>(a: Result<T, E>, b: Result<B, E>) -> Result<(T, B), E> {
    a.and_then(|a| b.map(|b| (a, b)))
  }
}

pub(crate) trait MapErrInto<T, E: Into<R>, R> {
  fn map_err_into(self) -> Result<T, R>;
}

impl<T, E: Into<R>, R> MapErrInto<T, E, R> for Result<T, E> {
  fn map_err_into(self) -> Result<T, R> {
    self.map_err(Into::into)
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

  fn perform_err(self, f: impl FnOnce(&E) -> ()) -> Result<T, E> {
    self.map_err(|t| {
          f(&t);
          t
        })
  }

  fn perform_mut(self, f: impl FnOnce(&mut T) -> ()) -> Result<T, E> {
    self.map(|mut t| {
          f(&mut t);
          t
        })
  }

  fn filter(self, pred: impl FnOnce(&T) -> bool, on_fail: impl FnOnce(&T) -> E) -> Result<T, E> {
    self.bind(|t| if pred(&t) { Err(on_fail(&t)) } else { Ok(t) })
  }
}
