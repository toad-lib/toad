/// Extensions to Result
pub trait ResultExt<T, E>: Sized {
  /// Alias for [`Result.and_then`]
  fn bind<R>(self, f: impl FnOnce(T) -> Result<R, E>) -> Result<R, E>;

  /// Allows turning an Err back into Ok by binding on the Err variant
  fn recover<R>(self, f: impl FnOnce(E) -> Result<T, R>) -> Result<T, R>;

  /// Attempt to perform some fallible IO
  fn try_perform(self, f: impl FnOnce(&T) -> Result<(), E>) -> Result<T, E>;

  /// Perform some IO when this Result is Err
  fn perform_err(self, f: impl FnOnce(&E) -> ()) -> Result<T, E>;

  /// Perform some IO when this Result is Ok
  fn perform(self, f: impl FnOnce(&T) -> ()) -> Result<T, E>;

  /// Perform some IO mutating the data contained in the Ok of this Result
  fn perform_mut(self, f: impl FnOnce(&mut T) -> ()) -> Result<T, E>;

  /// Test the data in Ok and turn it into an Err if it doesn't pass a predicate
  fn filter(self, pred: impl FnOnce(&T) -> bool, on_fail: impl FnOnce(&T) -> E) -> Result<T, E>;

  /// Do some fallible IO that resolves in a value and combine Oks
  fn tupled<R>(self, f: impl FnOnce(&T) -> Result<R, E>) -> Result<(T, R), E> {
    self.bind(|t| f(&t).map(|r| (t, r)))
  }

  /// Boolean AND
  fn two<B>(a: Result<T, E>, b: Result<B, E>) -> Result<(T, B), E> {
    a.and_then(|a| b.map(|b| (a, b)))
  }
}

impl<T, E> ResultExt<T, E> for Result<T, E> {
  fn bind<R>(self, f: impl FnOnce(T) -> Result<R, E>) -> Result<R, E> {
    self.and_then(f)
  }

  fn recover<R>(self, f: impl FnOnce(E) -> Result<T, R>) -> Result<T, R> {
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
