use core::marker::PhantomData;
use std::sync::RwLock;

use jni::objects::{JFieldID, JStaticFieldID};

use crate::java;

/// A high-level lens into a Java object field
pub struct Field<C, T> {
  name: &'static str,
  id: RwLock<Option<JFieldID>>,
  _t: PhantomData<(C, T)>,
}

impl<C, T> Field<C, T>
  where C: java::Class,
        T: java::Object
{
  /// Creates a new field lens
  pub const fn new(name: &'static str) -> Self {
    Self { name,
           id: RwLock::new(None),
           _t: PhantomData }
  }

  /// Get the value of this field
  pub fn get(&self, e: &mut java::Env, inst: &C) -> T {
    let id = self.id.read().unwrap();
    if id.is_none() {
      drop(id);
      let mut id = self.id.write().unwrap();
      *id = Some(e.get_field_id(C::PATH, self.name, T::SIG).unwrap());
      self.get(e, inst)
    } else {
      let inst = inst.downcast_ref(e);
      let val = e.get_field_unchecked(&inst, id.unwrap(), T::SIG.return_type())
                 .unwrap();
      T::upcast_value(e, val)
    }
  }

  /// Set the value of this field
  pub fn set(&self, e: &mut java::Env, inst: &C, t: T) {
    let inst = inst.downcast_ref(e);
    let t = t.downcast_value(e);
    e.set_field(inst, self.name, T::SIG, (&t).into()).unwrap();
  }
}

/// A high-level lens into a static Java object field
pub struct StaticField<C, T> {
  name: &'static str,
  id: RwLock<Option<JStaticFieldID>>,
  _t: PhantomData<(C, T)>,
}

impl<C, T> StaticField<C, T>
  where C: java::Class,
        T: java::Object
{
  /// Creates a new static field lens
  pub const fn new(name: &'static str) -> Self {
    Self { name,
           id: RwLock::new(None),
           _t: PhantomData }
  }

  /// Get the static field value
  pub fn get(&self, e: &mut java::Env) -> T {
    let id = self.id.read().unwrap();
    if id.is_none() {
      drop(id);
      let mut id = self.id.write().unwrap();
      *id = Some(e.get_static_field_id(C::PATH, self.name, T::SIG).unwrap());
      self.get(e)
    } else {
      let val = e.get_static_field_unchecked(C::PATH, id.unwrap(), T::jni())
                 .unwrap();
      T::upcast_value(e, val)
    }
  }
}
