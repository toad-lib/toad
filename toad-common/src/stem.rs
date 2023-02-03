use core::ops::{Deref, DerefMut};

#[cfg(feature = "std")]
type Inner<T> = std::sync::RwLock<T>;

#[cfg(not(feature = "std"))]
type Inner<T> = core::cell::RefCell<T>;

/// A thread-safe mutable memory location that allows
/// for many concurrent readers or a single writer.
///
/// When feature `std` enabled, this uses [`std::sync::RwLock`].
/// When `std` disabled, uses [`core::cell::Cell`].
#[derive(Debug, Default)]
pub struct Stem<T>(Inner<T>);

impl<T> Stem<T> {
  /// Create a new Stem cell
  pub const fn new(t: T) -> Self {
    Self(Inner::new(t))
  }

  /// Map a reference to `T` to a new type
  ///
  /// This will block if called concurrently with `map_mut`.
  ///
  /// There can be any number of concurrent `map_ref`
  /// sections running at a given time.
  pub fn map_ref<F, R>(&self, f: F) -> R
    where F: for<'a> FnMut(&'a T) -> R
  {
    self.0.map_ref(f)
  }

  /// Map a mutable reference to `T` to a new type
  ///
  /// This will block if called concurrently with `map_ref` or `map_mut`.
  pub fn map_mut<F, R>(&self, f: F) -> R
    where F: for<'a> FnMut(&'a mut T) -> R
  {
    self.0.map_mut(f)
  }
}

// NOTE(orion): I chose to use a trait here to tie RwLock
// and Cell together in a testable way, to keep the actual
// code behind feature flags extremely thin.

/// A mutable memory location
///
/// This is used to back the behavior of [`Stem`],
/// which should be used instead of this trait.
pub trait StemCellBehavior<T> {
  /// Create an instance of `Self`
  fn new(t: T) -> Self
    where Self: Sized;

  /// Map a reference to `T` to a new type
  ///
  /// Implementors may choose to panic or block
  /// if `map_mut` called concurrently.
  fn map_ref<F, R>(&self, f: F) -> R
    where F: for<'a> FnMut(&'a T) -> R;

  /// Map a mutable reference to `T` to a new type
  ///
  /// Implementors may choose to panic or block
  /// if `map_ref` or `map_mut` called concurrently.
  fn map_mut<F, R>(&self, f: F) -> R
    where F: for<'a> FnMut(&'a mut T) -> R;
}

#[cfg(feature = "std")]
impl<T> StemCellBehavior<T> for std::sync::RwLock<T> {
  fn new(t: T) -> Self {
    Self::new(t)
  }

  fn map_ref<F, R>(&self, mut f: F) -> R
    where F: for<'a> FnMut(&'a T) -> R
  {
    f(self.read().unwrap().deref())
  }

  fn map_mut<F, R>(&self, mut f: F) -> R
    where F: for<'a> FnMut(&'a mut T) -> R
  {
    f(self.write().unwrap().deref_mut())
  }
}

impl<T> StemCellBehavior<T> for core::cell::RefCell<T> {
  fn new(t: T) -> Self {
    Self::new(t)
  }

  fn map_ref<F, R>(&self, mut f: F) -> R
    where F: for<'a> FnMut(&'a T) -> R
  {
    f(self.borrow().deref())
  }

  fn map_mut<F, R>(&self, mut f: F) -> R
    where F: for<'a> FnMut(&'a mut T) -> R
  {
    f(self.borrow_mut().deref_mut())
  }
}

#[cfg(test)]
mod test {
  use core::cell::RefCell;
  use std::sync::{Arc, Barrier, RwLock};

  use super::*;

  #[test]
  fn refcell_modify() {
    let s = RefCell::new(Vec::<usize>::new());
    s.map_mut(|v| v.push(12));
    s.map_ref(|v| assert_eq!(v, &vec![12usize]));
  }

  #[test]
  fn refcell_concurrent_read_does_not_panic() {
    let s = RefCell::new(Vec::<usize>::new());
    s.map_ref(|_| s.map_ref(|_| ()));
  }

  #[test]
  fn rwlock_modify() {
    let s = RwLock::new(Vec::<usize>::new());
    s.map_mut(|v| v.push(12));
    s.map_ref(|v| assert_eq!(v, &vec![12usize]));
  }

  #[test]
  fn rwlock_concurrent_read_does_not_panic() {
    let s = RwLock::new(Vec::<usize>::new());
    s.map_ref(|_| s.map_ref(|_| ()));
  }

  #[test]
  fn stem_modify_blocks_until_refs_dropped() {
    unsafe {
      static VEC: Stem<Vec<usize>> = Stem::new(Vec::new());

      static mut START: Option<Arc<Barrier>> = None;
      static mut READING: Option<Arc<Barrier>> = None;
      static mut READING_DONE: Option<Arc<Barrier>> = None;
      static mut MODIFY_DONE: Option<Arc<Barrier>> = None;

      START = Some(Arc::new(Barrier::new(3)));
      READING = Some(Arc::new(Barrier::new(3)));
      READING_DONE = Some(Arc::new(Barrier::new(2)));
      MODIFY_DONE = Some(Arc::new(Barrier::new(3)));

      macro_rules! wait {
        ($b:ident) => {
          $b.as_ref().unwrap().clone().wait();
        };
      }

      std::thread::spawn(|| {
        wait!(START);
        VEC.map_ref(|v| {
             assert!(v.is_empty());
             wait!(READING);
             wait!(READING_DONE);
           });

        wait!(MODIFY_DONE);
      });

      std::thread::spawn(|| {
        wait!(START);
        wait!(READING);
        VEC.map_mut(|v| v.push(12)); // unblocked by READING_DONE
        wait!(MODIFY_DONE);
      });

      wait!(START);
      wait!(READING);
      VEC.map_ref(|v| assert!(v.is_empty()));

      wait!(READING_DONE);
      wait!(MODIFY_DONE);
      VEC.map_ref(|v| assert_eq!(v, &vec![12]));
    }
  }
}
