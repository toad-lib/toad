macro_rules! wrapper {
  (
    #[doc = $doc:expr]
    class $cls:ident {
      static $cls_:ident valueOf($inner_ty:ty);
      ($inner_ty2:ty) $inner_id:ident();
    }
  ) => {
    #[doc = $doc]
    pub struct $cls($crate::java::lang::Object);
    impl $cls {
      #[doc = concat!("Construct a new ", stringify!($cls))]
      pub fn new<'local>(e: &mut $crate::java::Env<'local>, b: $inner_ty) -> Self {
        use $crate::java::Class;

        let cls = e.find_class(Self::PATH).unwrap();
        let obj = e.call_static_method(cls,
                                       "valueOf",
                                       $crate::java::Signature::of::<fn($inner_ty) -> Self>(),
                                       &[b.into()])
                   .unwrap()
                   .l()
                   .unwrap();
        Self($crate::java::lang::Object::from_local(e, obj))
      }

      #[doc = concat!("yield the [`", stringify!($inner_ty), "`] value contained in this `", stringify!($cls), "` by invoking `", stringify!($cls), "#", stringify!($inner_id), "`")]
      pub fn inner<'local>(&self, e: &mut $crate::java::Env<'local>) -> $inner_ty {
        let jv = e.call_method(&self.0,
                               stringify!($inner_id),
                               $crate::java::Signature::of::<fn() -> $inner_ty>(),
                               &[])
                  .unwrap();
        <$inner_ty as $crate::java::Primitive>::from_jvalue(jv)
      }
    }

    impl $crate::java::Class for $cls {
      const PATH: &'static str = concat!("java/lang/", stringify!($cls));
    }

    impl $crate::java::Object for $cls {
      fn upcast<'a, 'e>(_: &'a mut $crate::java::Env<'e>, jobj: $crate::java::lang::Object) -> Self {
        Self(jobj)
      }

      fn downcast<'a, 'e>(self, _: &'a mut $crate::java::Env<'e>) -> $crate::java::lang::Object {
        self.0
      }

      fn downcast_ref<'a, 'e>(&'a self, e: &'a mut $crate::java::Env<'e>) -> $crate::java::lang::Object {
        (&self.0).downcast_ref(e)
      }
    }
  };
}

wrapper! {
  #[doc = "`java.lang.Byte`"]
  class Byte {
    static Byte valueOf(i8);
    (i8) byteValue();
  }
}

wrapper! {
  #[doc = "`java.lang.Short`"]
  class Short {
    static Short valueOf(i16);
    (i16) shortValue();
  }
}

wrapper! {
  #[doc = "`java.lang.Integer`"]
  class Integer {
    static Integer valueOf(i32);
    (i32) intValue();
  }
}

wrapper! {
  #[doc = "`java.lang.Long`"]
  class Long {
    static Long valueOf(i64);
    (i64) longValue();
  }
}

wrapper! {
  #[doc = "`java.lang.Float`"]
  class Float {
    static Float valueOf(f32);
    (f32) floatValue();
  }
}

wrapper! {
  #[doc = "`java.lang.Double`"]
  class Double {
    static Double valueOf(f64);
    (f64) doubleValue();
  }
}

wrapper! {
  #[doc = "`java.lang.Bool`"]
  class Bool {
    static Bool valueOf(bool);
    (bool) boolValue();
  }
}

wrapper! {
  #[doc = "`java.lang.Char`"]
  class Char {
    static Char valueOf(u16);
    (u16) boolValue();
  }
}
