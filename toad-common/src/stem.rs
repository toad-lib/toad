use core::ops::{Deref, DerefMut};

#[cfg(feature = "std")]
type Inner<T> = std::sync::RwLock<T>;

#[cfg(not(feature = "std"))]
type Inner<T> = core::cell::RefCell<T>;

/// A thread-safe mutable memory location that allows
/// for many concurrent readers or a single writer.
///
/// This is a wrapper of [`std::sync::RwLock`] that
/// switches to [`core::cell::Cell`] when feature `std`
/// is disabled.
///
/// # Naming
/// "Stem cell" is a pun, since stem cells in biology are
/// defined as cells which can mutate into any other kind
/// of cell, and this data structure will change its shape
/// based on the runtime.
#[derive(Debug, Default)]
pub struct Stem<T>(Inner<T>);

impl<T> Stem<T> {
  /// Create a new Stem cell
  pub const fn new(t: T) -> Self {
    Self(Inner::new(t))
  }

  /// Get a reference to T (`&'a T`)
  ///
  /// # Drop
  /// It is important that you drop the return value of
  /// this function as soon as possible, as usage of it
  /// will block calls to [`Stem::modify`]. (or cause it to panic)
  ///
  /// # Blocks
  /// When feature `std` enabled, this
  /// will block if a call to [`Stem::modify`]
  /// is running.
  ///
  /// # Panics
  /// When feature `std` disabled, this
  /// will panic if invoked while a call
  /// to [`Stem::modify`] is running.
  pub fn map_ref<F, R>(&self, f: F) -> R
    where F: for<'a> FnMut(&'a T) -> R
  {
    self.0.map_ref(f)
  }

  /// Modify `T`
  ///
  /// # Blocks
  /// When feature `std` enabled, this
  /// will block until all issued references
  /// from [`Stem::get_ref`] are dropped.
  ///
  /// # Panics
  /// When feature `std` disabled, this
  /// will panic if invoked when an issued
  /// reference from [`Stem::get_ref`] exists.
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

  /// Get a reference to `T` contained in `Self`
  ///
  /// # Panics
  /// Implementors may choose to panic (or block)
  /// if `get_ref` invoked while a [`StemCellBehavior::modify`]
  /// is running.
  fn map_ref<F, R>(&self, f: F) -> R
    where F: for<'a> FnMut(&'a T) -> R;

  /// Mutate the `T` contained in `Self`
  ///
  /// # Panics
  /// Implementors may choose to panic (or block)
  /// if `modify` invoked when [`StemCellBehavior::ReadLock`]s
  /// have been issued.
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
  use core::ptr::NonNull;
  use std::sync::{Arc, Barrier, RwLock};

  use super::*;

  #[test]
  fn refcell_modify() {
    let s = RefCell::new(Vec::<usize>::new());
    s.map_mut(|v| v.push(12));
    s.map_ref(|v| assert_eq!(v, &vec![12usize]));
  }

  #[test]
  fn rwlock_modify() {
    let s = RwLock::new(Vec::<usize>::new());
    s.map_mut(|v| v.push(12));
    s.map_ref(|v| assert_eq!(v, &vec![12usize]));
  }

  #[test]
  fn stem_modify_blocks_until_refs_dropped() {
    // NOTE: this test would panic on no_std!!
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
        VEC.map_ref(|v| {
             wait!(START);
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
