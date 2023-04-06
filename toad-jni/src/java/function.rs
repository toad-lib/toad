use core::marker::PhantomData;

use java::{Class, Object, Signature, Type};

use crate::java;

/// Compile-time lens into a method on a java class
///
/// For a simple example, see the source for [`java::math::BigInteger`]
pub struct Method<C, F> {
  name: &'static str,
  _t: PhantomData<(C, F)>,
}

impl<C, F> Method<C, F>
  where F: Type,
        C: Class
{
  /// Create a new method lens
  pub const fn new(name: &'static str) -> Self {
    Self { name,
           _t: PhantomData }
  }
}

impl<C, FR> Method<C, fn() -> FR>
  where C: Class,
        FR: Object
{
  /// Call the method
  pub fn invoke<'a, 'b>(&self, e: &mut java::Env<'a>, inst: &C) -> FR {
    let inst = inst.downcast_ref(e);
    let jv = e.call_method(&inst, self.name, Signature::of::<fn() -> FR>(), &[])
              .unwrap();

    FR::upcast_value(e, jv)
  }
}

impl<C, FA, FR> Method<C, fn(FA) -> FR>
  where C: Class,
        FA: Object,
        FR: Object
{
  /// Call the method
  pub fn invoke<'a, 'b>(&self, e: &mut java::Env<'a>, inst: &C, fa: FA) -> FR {
    let inst = inst.downcast_ref(e);
    let fa = fa.downcast_value(e);
    let jv = e.call_method(&inst,
                           self.name,
                           Signature::of::<fn(FA) -> FR>(),
                           &[(&fa).into()])
              .unwrap();
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
  pub fn invoke<'a, 'b>(&self, e: &mut java::Env<'a>, inst: &C, fa: FA, fb: FB) -> FR {
    let inst = inst.downcast_ref(e);
    let (fa, fb) = (fa.downcast_value(e), fb.downcast_value(e));
    let jv = e.call_method(&inst,
                           self.name,
                           Signature::of::<fn(FA, FB) -> FR>(),
                           &[(&fa).into(), (&fb).into()])
              .unwrap();
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
  pub fn invoke<'a, 'b>(&self, e: &mut java::Env<'a>, inst: &C, fa: FA, fb: FB, fc: FC) -> FR {
    let inst = inst.downcast_ref(e);
    let (fa, fb, fc) = (fa.downcast_value(e), fb.downcast_value(e), fc.downcast_value(e));
    let jv = e.call_method(&inst,
                           self.name,
                           Signature::of::<fn(FA, FB, FC) -> FR>(),
                           &[(&fa).into(), (&fb).into(), (&fc).into()])
              .unwrap();
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
  pub fn invoke<'a, 'b>(&self, e: &mut java::Env<'a>, inst: &C, fa: FA, fb: FB, fc: FC, fd: FD) -> FR {
    let inst = inst.downcast_ref(e);
    let (fa, fb, fc, fd) =
      (fa.downcast_value(e), fb.downcast_value(e), fc.downcast_value(e), fd.downcast_value(e));
    let jv = e.call_method(&inst,
                           self.name,
                           Signature::of::<fn(FA, FB, FC, FD) -> FR>(),
                           &[(&fa).into(), (&fb).into(), (&fc).into(), (&fd).into()])
              .unwrap();
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
  pub fn invoke<'a, 'b>(&self,
                        e: &mut java::Env<'a>,
                        inst: &C,
                        fa: FA,
                        fb: FB,
                        fc: FC,
                        fd: FD,
                        fe: FE)
                        -> FR {
    let inst = inst.downcast_ref(e);
    let (fa, fb, fc, fd, fe) = (fa.downcast_value(e),
                                fb.downcast_value(e),
                                fc.downcast_value(e),
                                fd.downcast_value(e),
                                fe.downcast_value(e));
    let jv = e.call_method(&inst,
                           self.name,
                           Signature::of::<fn(FA, FB, FC, FD, FE) -> FR>(),
                           &[(&fa).into(),
                             (&fb).into(),
                             (&fc).into(),
                             (&fd).into(),
                             (&fe).into()])
              .unwrap();
    FR::upcast_value(e, jv)
  }
}

/// Compile-time lens into a method on a java class
///
/// For a simple example, see the source for [`java::time::Duration`]
pub struct StaticMethod<C, F> {
  name: &'static str,
  _t: PhantomData<(C, F)>,
}

impl<C, F> StaticMethod<C, F>
  where F: Type,
        C: Class
{
  /// Create the static method lens
  pub const fn new(name: &'static str) -> Self {
    Self { name,
           _t: PhantomData }
  }
}

impl<C, FR> StaticMethod<C, fn() -> FR>
  where C: Class,
        FR: Object
{
  /// Invoke the static method
  pub fn invoke<'a, 'b>(&self, e: &mut java::Env<'a>) -> FR {
    let jv = e.call_static_method(C::PATH, self.name, Signature::of::<fn() -> FR>(), &[])
              .unwrap();
    FR::upcast_value(e, jv)
  }
}

impl<C, FA, FR> StaticMethod<C, fn(FA) -> FR>
  where C: Class,
        FA: Object,
        FR: Object
{
  /// Invoke the static method
  pub fn invoke<'a, 'b>(&self, e: &mut java::Env<'a>, fa: FA) -> FR {
    let fa = fa.downcast_value(e);
    let jv = e.call_static_method(C::PATH,
                                  self.name,
                                  Signature::of::<fn(FA) -> FR>(),
                                  &[(&fa).into()])
              .unwrap();
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
  pub fn invoke<'a, 'b>(&self, e: &mut java::Env<'a>, fa: FA, fb: FB) -> FR {
    let (fa, fb) = (fa.downcast_value(e), fb.downcast_value(e));
    let jv = e.call_static_method(C::PATH,
                                  self.name,
                                  Signature::of::<fn(FA, FB) -> FR>(),
                                  &[(&fa).into(), (&fb).into()])
              .unwrap();
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
  pub fn invoke<'a, 'b>(&self, e: &mut java::Env<'a>, fa: FA, fb: FB, fc: FC) -> FR {
    let (fa, fb, fc) = (fa.downcast_value(e), fb.downcast_value(e), fc.downcast_value(e));
    let jv = e.call_static_method(C::PATH,
                                  self.name,
                                  Signature::of::<fn(FA, FB, FC) -> FR>(),
                                  &[(&fa).into(), (&fb).into(), (&fc).into()])
              .unwrap();
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
  pub fn invoke<'a, 'b>(&self, e: &mut java::Env<'a>, fa: FA, fb: FB, fc: FC, fd: FD) -> FR {
    let (fa, fb, fc, fd) =
      (fa.downcast_value(e), fb.downcast_value(e), fc.downcast_value(e), fd.downcast_value(e));
    let jv = e.call_static_method(C::PATH,
                                  self.name,
                                  Signature::of::<fn(FA, FB, FC, FD) -> FR>(),
                                  &[(&fa).into(), (&fb).into(), (&fc).into(), (&fd).into()])
              .unwrap();
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
  pub fn invoke<'a, 'b>(&self, e: &mut java::Env<'a>, fa: FA, fb: FB, fc: FC, fd: FD, fe: FE) -> FR {
    let (fa, fb, fc, fd, fe) = (fa.downcast_value(e),
                                fb.downcast_value(e),
                                fc.downcast_value(e),
                                fd.downcast_value(e),
                                fe.downcast_value(e));
    let jv = e.call_static_method(C::PATH,
                                  self.name,
                                  Signature::of::<fn(FA, FB, FC, FD, FE) -> FR>(),
                                  &[(&fa).into(),
                                    (&fb).into(),
                                    (&fc).into(),
                                    (&fd).into(),
                                    (&fe).into()])
              .unwrap();
    FR::upcast_value(e, jv)
  }
}

/// A java class constructor lens
pub struct Constructor<C, F> {
  _t: PhantomData<(C, F)>,
}

impl<C, F> Constructor<C, F>
  where F: Type,
        C: Class
{
  /// Creates the lens
  pub const fn new() -> Self {
    Self { _t: PhantomData }
  }
}

impl<C> Constructor<C, fn()> where C: Class
{
  /// Invoke the constructor
  pub fn invoke<'a, 'b>(&self, e: &mut java::Env<'a>) -> C {
    let jobj = e.new_object(C::PATH, Signature::of::<fn()>(), &[]).unwrap();
    java::lang::Object::from_local(e, jobj).upcast_to::<C>(e)
  }
}

impl<C, FA> Constructor<C, fn(FA)>
  where C: Class,
        FA: Object
{
  /// Invoke the constructor
  pub fn invoke<'a, 'b>(&self, e: &mut java::Env<'a>, fa: FA) -> C {
    let fa = fa.downcast_value(e);
    let jv = e.new_object(C::PATH, Signature::of::<fn(FA)>(), &[(&fa).into()])
              .unwrap();

    java::lang::Object::from_local(e, jv).upcast_to::<C>(e)
  }
}

impl<C, FA, FB> Constructor<C, fn(FA, FB)>
  where C: Class,
        FA: Object,
        FB: Object
{
  /// Invoke the constructor
  pub fn invoke<'a, 'b>(&self, e: &mut java::Env<'a>, fa: FA, fb: FB) -> C {
    let (fa, fb) = (fa.downcast_value(e), fb.downcast_value(e));
    let jv = e.new_object(C::PATH,
                          Signature::of::<fn(FA, FB)>(),
                          &[(&fa).into(), (&fb).into()])
              .unwrap();
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
  pub fn invoke<'a, 'b>(&self, e: &mut java::Env<'a>, fa: FA, fb: FB, fc: FC) -> C {
    let (fa, fb, fc) = (fa.downcast_value(e), fb.downcast_value(e), fc.downcast_value(e));
    let jv = e.new_object(C::PATH,
                          Signature::of::<fn(FA, FB, FC)>(),
                          &[(&fa).into(), (&fb).into(), (&fc).into()])
              .unwrap();
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
  pub fn invoke<'a, 'b>(&self, e: &mut java::Env<'a>, fa: FA, fb: FB, fc: FC, fd: FD) -> C {
    let (fa, fb, fc, fd) =
      (fa.downcast_value(e), fb.downcast_value(e), fc.downcast_value(e), fd.downcast_value(e));
    let jv = e.new_object(C::PATH,
                          Signature::of::<fn(FA, FB, FC, FD)>(),
                          &[(&fa).into(), (&fb).into(), (&fc).into(), (&fd).into()])
              .unwrap();
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
  pub fn invoke<'a, 'b>(&self, e: &mut java::Env<'a>, fa: FA, fb: FB, fc: FC, fd: FD, fe: FE) -> C {
    let (fa, fb, fc, fd, fe) = (fa.downcast_value(e),
                                fb.downcast_value(e),
                                fc.downcast_value(e),
                                fd.downcast_value(e),
                                fe.downcast_value(e));
    let jv = e.new_object(C::PATH,
                          Signature::of::<fn(FA, FB, FC, FD, FE)>(),
                          &[(&fa).into(),
                            (&fb).into(),
                            (&fc).into(),
                            (&fd).into(),
                            (&fe).into()])
              .unwrap();
    java::lang::Object::from_local(e, jv).upcast_to::<C>(e)
  }
}
