use jni::objects::GlobalRef;

use crate::{global, Sig};

macro_rules! wrapper {
  (
    #[doc = $doc:expr]
    class $cls:ident {
      static $cls_:ident valueOf($inner_sig:expr);
      ($inner_ty:ty) $inner_id:ident();
    }
  ) => {
    #[doc = $doc]
    pub struct $cls(GlobalRef);
    impl $cls {
      const ID: &'static str = concat!("java/lang/", stringify!($cls));

      #[doc = concat!("Construct a new ", stringify!($cls))]
      pub fn new(b: $inner_ty) -> Self {
        let mut e = global::env();
        let cls = e.find_class(Self::ID).unwrap();
        let obj = e.call_static_method(cls,
                                       "valueOf",
                                       Sig::new().arg($inner_sig).returning(Sig::class(Self::ID)),
                                       &[b.into()])
                   .unwrap()
                   .l()
                   .unwrap();
        let obj = e.new_global_ref(obj).unwrap();
        Self(obj)
      }

      #[doc = concat!("yield the [`", stringify!($inner_ty), "`] value contained in this `", stringify!($cls), "` by invoking `", stringify!($cls), "#", stringify!($inner_id), "`")]
      pub fn inner(&self) -> $inner_ty {
        let mut e = global::env();
        let jv = e.call_method(&self.0,
                               stringify!($inner_id),
                               Sig::new().returning($inner_sig),
                               &[])
                  .unwrap();
        <$inner_ty as $crate::convert::Primitive>::from_jvalue(jv)
      }
    }

    impl<'local> $crate::convert::Object for $cls {
      fn from_java(jobj: jni::objects::GlobalRef) -> Self {
        Self(jobj)
      }

      fn to_java(self) -> jni::objects::GlobalRef {
        self.0
      }
    }
  };
}

wrapper! {
  #[doc = "java/lang/Byte"]
  class Byte {
    static Byte valueOf(Sig::BYTE);
    (i8) byteValue();
  }
}

wrapper! {
  #[doc = "java/lang/Short"]
  class Short {
    static Short valueOf(Sig::SHORT);
    (i16) shortValue();
  }
}

wrapper! {
  #[doc = "java/lang/Int"]
  class Int {
    static Int valueOf(Sig::INT);
    (i32) intValue();
  }
}

wrapper! {
  #[doc = "java/lang/Long"]
  class Long {
    static Long valueOf(Sig::LONG);
    (i64) longValue();
  }
}

wrapper! {
  #[doc = "java/lang/Float"]
  class Float {
    static Float valueOf(Sig::FLOAT);
    (f32) floatValue();
  }
}

wrapper! {
  #[doc = "java/lang/Double"]
  class Double {
    static Double valueOf(Sig::DOUBLE);
    (f64) doubleValue();
  }
}

wrapper! {
  #[doc = "java/lang/Bool"]
  class Bool {
    static Bool valueOf(Sig::BOOL);
    (bool) boolValue();
  }
}
