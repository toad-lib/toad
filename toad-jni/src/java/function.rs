#![allow(clippy::too_many_arguments)]

use core::marker::PhantomData;
use std::sync::RwLock;

use java::{Class, Object, ResultExt, Signature, Type};
use jni::objects::{GlobalRef, JClass, JMethodID, JObject, JStaticMethodID};

use crate::java;

/// Static high-level wrapper of Java class instance methods
///
/// See the [module documentation](crate::java) for examples.
pub struct Method<C, F> {
  name: &'static str,
  mid: RwLock<Option<JMethodID>>,
  _t: PhantomData<(C, F)>,
}

impl<C, F> Method<C, F>
  where F: Type,
        C: Class
{
  /// Create a new method lens that invokes the method named `name` on class `C`.
  ///
  /// If you want to invoke a potentially overridden method definition on the
  /// object instances, use [`Method::new_overrideable`]
  pub const fn new(name: &'static str) -> Self {
    Self { name,
           mid: RwLock::new(None),
           _t: PhantomData }
  }

  /// Get & cache the method ID for this method
  fn find(&self, e: &mut java::Env) -> JMethodID {
    let mid = self.mid.read().unwrap();

    if mid.is_none() {
      drop(mid);
      let mid = e.get_method_id(C::PATH, self.name, F::SIG).unwrap_java(e);
      let mut field = self.mid.write().unwrap();
      *field = Some(mid);
      mid
    } else {
      mid.unwrap()
    }
  }
}

impl<C, FR> Method<C, fn() -> FR>
  where C: Class,
        FR: Object
{
  /// Call the method
  pub fn invoke(&self, e: &mut java::Env, inst: &C) -> FR {
    let inst = inst.downcast_ref(e);
    let mid = self.find(e);
    let jv = unsafe {
      e.call_method_unchecked(&inst, mid, Signature::of::<fn() -> FR>().return_type(), &[])
       .unwrap_java(e)
    };

    FR::upcast_value(e, jv)
  }
}

impl<C, FA, FR> Method<C, fn(FA) -> FR>
  where C: Class,
        FA: Object,
        FR: Object
{
  /// Call the method
  pub fn invoke(&self, e: &mut java::Env, inst: &C, fa: FA) -> FR {
    let inst = inst.downcast_ref(e);
    let fa = fa.downcast_value(e);
    let mid = self.find(e);
    let jv = unsafe {
      e.call_method_unchecked(&inst,
                              mid,
                              Signature::of::<fn(FA) -> FR>().return_type(),
                              &[fa.as_jni()])
       .unwrap_java(e)
    };
    FR::upcast_value(e, jv)
  }
}

impl<C, FA, FB, FR> Method<C, fn(FA, FB) -> FR>
  where C: Class,
        FA: Object,
        FB: Object,
        FR: Object
{
  /// Call the method
  pub fn invoke(&self, e: &mut java::Env, inst: &C, fa: FA, fb: FB) -> FR {
    let inst = inst.downcast_ref(e);
    let (fa, fb) = (fa.downcast_value(e), fb.downcast_value(e));
    let mid = self.find(e);
    let jv = unsafe {
      e.call_method_unchecked(&inst,
                              mid,
                              Signature::of::<fn(FA, FB) -> FR>().return_type(),
                              &[fa.as_jni(), fb.as_jni()])
       .unwrap_java(e)
    };
    FR::upcast_value(e, jv)
  }
}

impl<C, FA, FB, FC, FR> Method<C, fn(FA, FB, FC) -> FR>
  where C: Class,
        FA: Object,
        FB: Object,
        FC: Object,
        FR: Object
{
  /// Call the method
  pub fn invoke(&self, e: &mut java::Env, inst: &C, fa: FA, fb: FB, fc: FC) -> FR {
    let inst = inst.downcast_ref(e);
    let (fa, fb, fc) = (fa.downcast_value(e), fb.downcast_value(e), fc.downcast_value(e));
    let mid = self.find(e);
    let jv = unsafe {
      e.call_method_unchecked(&inst,
                              mid,
                              Signature::of::<fn(FA, FB, FC) -> FR>().return_type(),
                              &[fa.as_jni(), fb.as_jni(), fc.as_jni()])
       .unwrap_java(e)
    };
    FR::upcast_value(e, jv)
  }
}

impl<C, FA, FB, FC, FD, FR> Method<C, fn(FA, FB, FC, FD) -> FR>
  where C: Class,
        FA: Object,
        FB: Object,
        FC: Object,
        FD: Object,
        FR: Object
{
  /// Call the method
  pub fn invoke(&self, e: &mut java::Env, inst: &C, fa: FA, fb: FB, fc: FC, fd: FD) -> FR {
    let inst = inst.downcast_ref(e);
    let (fa, fb, fc, fd) =
      (fa.downcast_value(e), fb.downcast_value(e), fc.downcast_value(e), fd.downcast_value(e));
    let mid = self.find(e);
    let jv = unsafe {
      e.call_method_unchecked(&inst,
                              mid,
                              Signature::of::<fn(FA, FB, FC, FD) -> FR>().return_type(),
                              &[fa.as_jni(), fb.as_jni(), fc.as_jni(), fd.as_jni()])
       .unwrap_java(e)
    };
    FR::upcast_value(e, jv)
  }
}

impl<C, FA, FB, FC, FD, FE, FR> Method<C, fn(FA, FB, FC, FD, FE) -> FR>
  where C: Class,
        FA: Object,
        FB: Object,
        FC: Object,
        FD: Object,
        FE: Object,
        FR: Object
{
  /// Call the method
  pub fn invoke(&self, e: &mut java::Env, inst: &C, fa: FA, fb: FB, fc: FC, fd: FD, fe: FE) -> FR {
    let inst = inst.downcast_ref(e);
    let (fa, fb, fc, fd, fe) = (fa.downcast_value(e),
                                fb.downcast_value(e),
                                fc.downcast_value(e),
                                fd.downcast_value(e),
                                fe.downcast_value(e));
    let mid = self.find(e);
    let jv = unsafe {
      e.call_method_unchecked(&inst,
                              mid,
                              Signature::of::<fn(FA, FB, FC, FD, FE) -> FR>().return_type(),
                              &[fa.as_jni(),
                                fb.as_jni(),
                                fc.as_jni(),
                                fd.as_jni(),
                                fe.as_jni()])
       .unwrap_java(e)
    };
    FR::upcast_value(e, jv)
  }
}

impl<C, FR> Method<C, fn() -> Result<FR, java::lang::Throwable>>
  where C: Class,
        FR: Class
{
  /// Call the method
  pub fn invoke(&self, e: &mut java::Env, inst: &C) -> Result<FR, java::lang::Throwable> {
    let inst = inst.downcast_ref(e);
    let mid = self.find(e);
    unsafe {
      e.call_method_unchecked(&inst, mid, Signature::of::<fn() -> FR>().return_type(), &[])
       .to_throwable(e)
       .map(|jv| FR::upcast_value(e, jv))
    }
  }
}

impl<C, FA, FR> Method<C, fn(FA) -> Result<FR, java::lang::Throwable>>
  where C: Class,
        FA: Object,
        FR: Class
{
  /// Call the method
  pub fn invoke(&self, e: &mut java::Env, inst: &C, fa: FA) -> Result<FR, java::lang::Throwable> {
    let inst = inst.downcast_ref(e);
    let fa = fa.downcast_value(e);
    let mid = self.find(e);
    unsafe {
      e.call_method_unchecked(&inst,
                              mid,
                              Signature::of::<fn(FA) -> FR>().return_type(),
                              &[fa.as_jni()])
       .to_throwable(e)
       .map(|jv| FR::upcast_value(e, jv))
    }
  }
}

impl<C, FA, FB, FR> Method<C, fn(FA, FB) -> Result<FR, java::lang::Throwable>>
  where C: Class,
        FA: Object,
        FB: Object,
        FR: Class
{
  /// Call the method
  pub fn invoke(&self,
                e: &mut java::Env,
                inst: &C,
                fa: FA,
                fb: FB)
                -> Result<FR, java::lang::Throwable> {
    let inst = inst.downcast_ref(e);
    let (fa, fb) = (fa.downcast_value(e), fb.downcast_value(e));
    let mid = self.find(e);
    unsafe {
      e.call_method_unchecked(&inst,
                              mid,
                              Signature::of::<fn(FA, FB) -> FR>().return_type(),
                              &[fa.as_jni(), fb.as_jni()])
       .to_throwable(e)
       .map(|jv| FR::upcast_value(e, jv))
    }
  }
}

impl<C, FA, FB, FC, FR> Method<C, fn(FA, FB, FC) -> Result<FR, java::lang::Throwable>>
  where C: Class,
        FA: Object,
        FB: Object,
        FC: Object,
        FR: Class
{
  /// Call the method
  pub fn invoke(&self,
                e: &mut java::Env,
                inst: &C,
                fa: FA,
                fb: FB,
                fc: FC)
                -> Result<FR, java::lang::Throwable> {
    let inst = inst.downcast_ref(e);
    let (fa, fb, fc) = (fa.downcast_value(e), fb.downcast_value(e), fc.downcast_value(e));
    let mid = self.find(e);
    unsafe {
      e.call_method_unchecked(&inst,
                              mid,
                              Signature::of::<fn(FA, FB, FC) -> FR>().return_type(),
                              &[fa.as_jni(), fb.as_jni(), fc.as_jni()])
       .to_throwable(e)
       .map(|jv| FR::upcast_value(e, jv))
    }
  }
}

impl<C, FA, FB, FC, FD, FR> Method<C, fn(FA, FB, FC, FD) -> Result<FR, java::lang::Throwable>>
  where C: Class,
        FA: Object,
        FB: Object,
        FC: Object,
        FD: Object,
        FR: Class
{
  /// Call the method
  pub fn invoke(&self,
                e: &mut java::Env,
                inst: &C,
                fa: FA,
                fb: FB,
                fc: FC,
                fd: FD)
                -> Result<FR, java::lang::Throwable> {
    let inst = inst.downcast_ref(e);
    let (fa, fb, fc, fd) =
      (fa.downcast_value(e), fb.downcast_value(e), fc.downcast_value(e), fd.downcast_value(e));
    let mid = self.find(e);
    unsafe {
      e.call_method_unchecked(&inst,
                              mid,
                              Signature::of::<fn(FA, FB, FC, FD) -> FR>().return_type(),
                              &[fa.as_jni(), fb.as_jni(), fc.as_jni(), fd.as_jni()])
       .to_throwable(e)
       .map(|jv| FR::upcast_value(e, jv))
    }
  }
}

impl<C, FA, FB, FC, FD, FE, FR>
  Method<C, fn(FA, FB, FC, FD, FE) -> Result<FR, java::lang::Throwable>>
  where C: Class,
        FA: Object,
        FB: Object,
        FC: Object,
        FD: Object,
        FE: Object,
        FR: Class
{
  /// Call the method
  pub fn invoke(&self,
                e: &mut java::Env,
                inst: &C,
                fa: FA,
                fb: FB,
                fc: FC,
                fd: FD,
                fe: FE)
                -> Result<FR, java::lang::Throwable> {
    let inst = inst.downcast_ref(e);
    let (fa, fb, fc, fd, fe) = (fa.downcast_value(e),
                                fb.downcast_value(e),
                                fc.downcast_value(e),
                                fd.downcast_value(e),
                                fe.downcast_value(e));
    let mid = self.find(e);
    unsafe {
      e.call_method_unchecked(&inst,
                              mid,
                              Signature::of::<fn(FA, FB, FC, FD, FE) -> FR>().return_type(),
                              &[fa.as_jni(),
                                fb.as_jni(),
                                fc.as_jni(),
                                fd.as_jni(),
                                fe.as_jni()])
       .to_throwable(e)
       .map(|jv| FR::upcast_value(e, jv))
    }
  }
}

/// Static high-level wrapper of static Java class methods
///
/// See the [module documentation](crate::java) for examples.
pub struct StaticMethod<C, F> {
  name: &'static str,
  ids: RwLock<Option<(GlobalRef, JStaticMethodID)>>,
  _t: PhantomData<(C, F)>,
}

impl<C, F> StaticMethod<C, F>
  where F: Type,
        C: Class
{
  /// Create the static method lens
  pub const fn new(name: &'static str) -> Self {
    Self { name,
           ids: RwLock::new(None),
           _t: PhantomData }
  }

  /// Get & cache the method ID for this method
  fn find(&self, e: &mut java::Env) -> (JClass, JStaticMethodID) {
    let ids = self.ids.read().unwrap();

    if ids.is_none() {
      drop(ids);
      let class = e.find_class(C::PATH).unwrap_java(e);
      let class = e.new_global_ref(class).unwrap_java(e);
      let mid = e.get_static_method_id(C::PATH, self.name, F::SIG)
                 .unwrap_java(e);
      let mut field = self.ids.write().unwrap();
      *field = Some((class, mid));
      drop(field);
      self.find(e)
    } else {
      let (g, mid) = ids.as_ref().unwrap();

      // SAFETY: this reference never escapes this module and will not be wrapped in AutoLocal
      // (which is the only UB risk with casting a GlobalRef to an owned JObject)
      let jobj = unsafe { JObject::from_raw(g.as_obj().as_raw()) };

      (jobj.into(), *mid)
    }
  }
}

impl<C, FR> StaticMethod<C, fn() -> FR>
  where C: Class,
        FR: Object
{
  /// Invoke the static method
  pub fn invoke(&self, e: &mut java::Env) -> FR {
    let (class, mid) = self.find(e);
    let jv = unsafe {
      e.call_static_method_unchecked(class, mid, Signature::of::<fn() -> FR>().return_type(), &[])
       .unwrap_java(e)
    };
    FR::upcast_value(e, jv)
  }
}

impl<C, FA, FR> StaticMethod<C, fn(FA) -> FR>
  where C: Class,
        FA: Object,
        FR: Object
{
  /// Invoke the static method
  pub fn invoke(&self, e: &mut java::Env, fa: FA) -> FR {
    let fa = fa.downcast_value(e);
    let (class, mid) = self.find(e);
    let jv = unsafe {
      e.call_static_method_unchecked(class,
                                     mid,
                                     Signature::of::<fn(FA) -> FR>().return_type(),
                                     &[fa.as_jni()])
       .unwrap_java(e)
    };
    FR::upcast_value(e, jv)
  }
}

impl<C, FA, FB, FR> StaticMethod<C, fn(FA, FB) -> FR>
  where C: Class,
        FA: Object,
        FB: Object,
        FR: Object
{
  /// Invoke the static method
  pub fn invoke(&self, e: &mut java::Env, fa: FA, fb: FB) -> FR {
    let (fa, fb) = (fa.downcast_value(e), fb.downcast_value(e));
    let (class, mid) = self.find(e);
    let jv = unsafe {
      e.call_static_method_unchecked(class,
                                     mid,
                                     Signature::of::<fn(FA, FB) -> FR>().return_type(),
                                     &[fa.as_jni(), fb.as_jni()])
       .unwrap_java(e)
    };
    FR::upcast_value(e, jv)
  }
}

impl<C, FA, FB, FC, FR> StaticMethod<C, fn(FA, FB, FC) -> FR>
  where C: Class,
        FA: Object,
        FB: Object,
        FC: Object,
        FR: Object
{
  /// Invoke the static method
  pub fn invoke(&self, e: &mut java::Env, fa: FA, fb: FB, fc: FC) -> FR {
    let (fa, fb, fc) = (fa.downcast_value(e), fb.downcast_value(e), fc.downcast_value(e));
    let (class, mid) = self.find(e);
    let jv = unsafe {
      e.call_static_method_unchecked(class,
                                     mid,
                                     Signature::of::<fn(FA, FB, FC) -> FR>().return_type(),
                                     &[fa.as_jni(), fb.as_jni(), fc.as_jni()])
       .unwrap_java(e)
    };
    FR::upcast_value(e, jv)
  }
}

impl<C, FA, FB, FC, FD, FR> StaticMethod<C, fn(FA, FB, FC, FD) -> FR>
  where C: Class,
        FA: Object,
        FB: Object,
        FC: Object,
        FD: Object,
        FR: Object
{
  /// Invoke the static method
  pub fn invoke(&self, e: &mut java::Env, fa: FA, fb: FB, fc: FC, fd: FD) -> FR {
    let (fa, fb, fc, fd) =
      (fa.downcast_value(e), fb.downcast_value(e), fc.downcast_value(e), fd.downcast_value(e));
    let (class, mid) = self.find(e);
    let jv = unsafe {
      e.call_static_method_unchecked(class,
                                     mid,
                                     Signature::of::<fn(FA, FB, FC, FD) -> FR>().return_type(),
                                     &[fa.as_jni(), fb.as_jni(), fc.as_jni(), fd.as_jni()])
       .unwrap_java(e)
    };
    FR::upcast_value(e, jv)
  }
}

impl<C, FA, FB, FC, FD, FE, FR> StaticMethod<C, fn(FA, FB, FC, FD, FE) -> FR>
  where C: Class,
        FA: Object,
        FB: Object,
        FC: Object,
        FD: Object,
        FE: Object,
        FR: Object
{
  /// Invoke the static method
  pub fn invoke(&self, e: &mut java::Env, fa: FA, fb: FB, fc: FC, fd: FD, fe: FE) -> FR {
    let (fa, fb, fc, fd, fe) = (fa.downcast_value(e),
                                fb.downcast_value(e),
                                fc.downcast_value(e),
                                fd.downcast_value(e),
                                fe.downcast_value(e));
    let (class, mid) = self.find(e);
    let jv = unsafe {
      e.call_static_method_unchecked(class,
                                     mid,
                                     Signature::of::<fn(FA, FB, FC, FD, FE) -> FR>().return_type(),
                                     &[fa.as_jni(),
                                       fb.as_jni(),
                                       fc.as_jni(),
                                       fd.as_jni(),
                                       fe.as_jni()])
       .unwrap_java(e)
    };
    FR::upcast_value(e, jv)
  }
}

impl<C, FR> StaticMethod<C, fn() -> Result<FR, java::lang::Throwable>>
  where C: Class,
        FR: Class
{
  /// Invoke the static method
  pub fn invoke(&self, e: &mut java::Env) -> Result<FR, java::lang::Throwable> {
    let (class, mid) = self.find(e);
    unsafe {
      e.call_static_method_unchecked(class, mid, Signature::of::<fn() -> Result<FR, java::lang::Throwable>>().return_type(), &[])
       .to_throwable(e).map(|jv| FR::upcast_value(e, jv))
    }
  }
}

impl<C, FA, FR> StaticMethod<C, fn(FA) -> Result<FR, java::lang::Throwable>>
  where C: Class,
        FA: Object,
        FR: Class
{
  /// Invoke the static method
  pub fn invoke(&self, e: &mut java::Env, fa: FA) -> Result<FR, java::lang::Throwable> {
    let fa = fa.downcast_value(e);
    let (class, mid) = self.find(e);
    unsafe {
      e.call_static_method_unchecked(class,
                                     mid,
                                     Signature::of::<fn(FA) -> Result<FR, java::lang::Throwable>>().return_type(),
                                     &[fa.as_jni()])
       .to_throwable(e).map(|jv| FR::upcast_value(e, jv))
    }
  }
}

impl<C, FA, FB, FR> StaticMethod<C, fn(FA, FB) -> Result<FR, java::lang::Throwable>>
  where C: Class,
        FA: Object,
        FB: Object,
        FR: Class
{
  /// Invoke the static method
  pub fn invoke(&self, e: &mut java::Env, fa: FA, fb: FB) -> Result<FR, java::lang::Throwable> {
    let (fa, fb) = (fa.downcast_value(e), fb.downcast_value(e));
    let (class, mid) = self.find(e);
    unsafe {
      e.call_static_method_unchecked(class,
                                     mid,
                                     Signature::of::<fn(FA, FB) -> Result<FR, java::lang::Throwable>>().return_type(),
                                     &[fa.as_jni(), fb.as_jni()])
       .to_throwable(e).map(|jv| FR::upcast_value(e, jv))
    }
  }
}

impl<C, FA, FB, FC, FR> StaticMethod<C, fn(FA, FB, FC) -> Result<FR, java::lang::Throwable>>
  where C: Class,
        FA: Object,
        FB: Object,
        FC: Object,
        FR: Class
{
  /// Invoke the static method
  pub fn invoke(&self,
                e: &mut java::Env,
                fa: FA,
                fb: FB,
                fc: FC)
                -> Result<FR, java::lang::Throwable> {
    let (fa, fb, fc) = (fa.downcast_value(e), fb.downcast_value(e), fc.downcast_value(e));
    let (class, mid) = self.find(e);
    unsafe {
      e.call_static_method_unchecked(class,
                                     mid,
                                     Signature::of::<fn(FA, FB, FC) -> Result<FR, java::lang::Throwable>>().return_type(),
                                     &[fa.as_jni(), fb.as_jni(), fc.as_jni()])
       .to_throwable(e).map(|jv| FR::upcast_value(e, jv))
    }
  }
}

impl<C, FA, FB, FC, FD, FR> StaticMethod<C, fn(FA, FB, FC, FD) -> Result<FR, java::lang::Throwable>>
  where C: Class,
        FA: Object,
        FB: Object,
        FC: Object,
        FD: Object,
        FR: Class
{
  /// Invoke the static method
  pub fn invoke(&self,
                e: &mut java::Env,
                fa: FA,
                fb: FB,
                fc: FC,
                fd: FD)
                -> Result<FR, java::lang::Throwable> {
    let (fa, fb, fc, fd) =
      (fa.downcast_value(e), fb.downcast_value(e), fc.downcast_value(e), fd.downcast_value(e));
    let (class, mid) = self.find(e);
    unsafe {
      e.call_static_method_unchecked(class,
                                     mid,
                                     Signature::of::<fn(FA, FB, FC, FD) -> Result<FR, java::lang::Throwable>>().return_type(),
                                     &[fa.as_jni(), fb.as_jni(), fc.as_jni(), fd.as_jni()])
       .to_throwable(e).map(|jv| FR::upcast_value(e, jv))
    }
  }
}

impl<C, FA, FB, FC, FD, FE, FR>
  StaticMethod<C, fn(FA, FB, FC, FD, FE) -> Result<FR, java::lang::Throwable>>
  where C: Class,
        FA: Object,
        FB: Object,
        FC: Object,
        FD: Object,
        FE: Object,
        FR: Class
{
  /// Invoke the static method
  pub fn invoke(&self,
                e: &mut java::Env,
                fa: FA,
                fb: FB,
                fc: FC,
                fd: FD,
                fe: FE)
                -> Result<FR, java::lang::Throwable> {
    let (fa, fb, fc, fd, fe) = (fa.downcast_value(e),
                                fb.downcast_value(e),
                                fc.downcast_value(e),
                                fd.downcast_value(e),
                                fe.downcast_value(e));
    let (class, mid) = self.find(e);
    unsafe {
      e.call_static_method_unchecked(class,
                                     mid,
                                     Signature::of::<fn(FA, FB, FC, FD, FE) -> Result<FR, java::lang::Throwable>>().return_type(),
                                     &[fa.as_jni(),
                                       fb.as_jni(),
                                       fc.as_jni(),
                                       fd.as_jni(),
                                       fe.as_jni()])
       .to_throwable(e).map(|jv| FR::upcast_value(e, jv))
    }
  }
}

/// A static high-level wrapper around Java class
/// constructors
///
/// See the [module documentation](crate::java) for examples.
pub struct Constructor<C, F> {
  id: RwLock<Option<JMethodID>>,
  _t: PhantomData<(C, F)>,
}

impl<C, F> Constructor<C, F>
  where F: Type,
        C: Class
{
  /// Creates the lens
  pub const fn new() -> Self {
    Self { id: RwLock::new(None),
           _t: PhantomData }
  }

  /// Get & cache the method ID for this constructor
  fn find(&self, e: &mut java::Env) -> JMethodID {
    let mid = self.id.read().unwrap();

    if mid.is_none() {
      drop(mid);
      let mid = e.get_method_id(C::PATH, "<init>", F::SIG).unwrap_java(e);
      let mut field = self.id.write().unwrap();
      *field = Some(mid);
      mid
    } else {
      mid.unwrap()
    }
  }
}

impl<C> Constructor<C, fn()> where C: Class
{
  /// Invoke the constructor
  pub fn invoke(&self, e: &mut java::Env) -> C {
    let jobj = e.new_object(C::PATH, Signature::of::<fn()>(), &[])
                .unwrap_java(e);
    java::lang::Object::from_local(e, jobj).upcast_to::<C>(e)
  }
}

impl<C, FA> Constructor<C, fn(FA)>
  where C: Class,
        FA: Object
{
  /// Invoke the constructor
  pub fn invoke(&self, e: &mut java::Env, fa: FA) -> C {
    let fa = fa.downcast_value(e);
    let mid = self.find(e);
    let jv = unsafe {
      e.new_object_unchecked(C::PATH, mid, &[fa.as_jni()])
       .unwrap_java(e)
    };

    java::lang::Object::from_local(e, jv).upcast_to::<C>(e)
  }
}

impl<C, FA, FB> Constructor<C, fn(FA, FB)>
  where C: Class,
        FA: Object,
        FB: Object
{
  /// Invoke the constructor
  pub fn invoke(&self, e: &mut java::Env, fa: FA, fb: FB) -> C {
    let (fa, fb) = (fa.downcast_value(e), fb.downcast_value(e));
    let mid = self.find(e);
    let jv = unsafe {
      e.new_object_unchecked(C::PATH, mid, &[fa.as_jni(), fb.as_jni()])
       .unwrap_java(e)
    };
    java::lang::Object::from_local(e, jv).upcast_to::<C>(e)
  }
}

impl<C, FA, FB, FC> Constructor<C, fn(FA, FB, FC)>
  where C: Class,
        FA: Object,
        FB: Object,
        FC: Object
{
  /// Invoke the constructor
  pub fn invoke(&self, e: &mut java::Env, fa: FA, fb: FB, fc: FC) -> C {
    let (fa, fb, fc) = (fa.downcast_value(e), fb.downcast_value(e), fc.downcast_value(e));
    let mid = self.find(e);
    let jv = unsafe {
      e.new_object_unchecked(C::PATH, mid, &[fa.as_jni(), fb.as_jni(), fc.as_jni()])
       .unwrap_java(e)
    };
    java::lang::Object::from_local(e, jv).upcast_to::<C>(e)
  }
}

impl<C, FA, FB, FC, FD> Constructor<C, fn(FA, FB, FC, FD)>
  where C: Class,
        FA: Object,
        FB: Object,
        FC: Object,
        FD: Object
{
  /// Invoke the constructor
  pub fn invoke(&self, e: &mut java::Env, fa: FA, fb: FB, fc: FC, fd: FD) -> C {
    let (fa, fb, fc, fd) =
      (fa.downcast_value(e), fb.downcast_value(e), fc.downcast_value(e), fd.downcast_value(e));
    let mid = self.find(e);
    let jv = unsafe {
      e.new_object_unchecked(C::PATH,
                             mid,
                             &[fa.as_jni(), fb.as_jni(), fc.as_jni(), fd.as_jni()])
       .unwrap_java(e)
    };
    java::lang::Object::from_local(e, jv).upcast_to::<C>(e)
  }
}

impl<C, FA, FB, FC, FD, FE> Constructor<C, fn(FA, FB, FC, FD, FE)>
  where C: Class,
        FA: Object,
        FB: Object,
        FC: Object,
        FD: Object,
        FE: Object
{
  /// Invoke the constructor
  pub fn invoke(&self, e: &mut java::Env, fa: FA, fb: FB, fc: FC, fd: FD, fe: FE) -> C {
    let (fa, fb, fc, fd, fe) = (fa.downcast_value(e),
                                fb.downcast_value(e),
                                fc.downcast_value(e),
                                fd.downcast_value(e),
                                fe.downcast_value(e));
    let mid = self.find(e);
    let jv = unsafe {
      e.new_object_unchecked(C::PATH,
                             mid,
                             &[fa.as_jni(),
                               fb.as_jni(),
                               fc.as_jni(),
                               fd.as_jni(),
                               fe.as_jni()])
       .unwrap_java(e)
    };
    java::lang::Object::from_local(e, jv).upcast_to::<C>(e)
  }
}
