[![crates.io](https://img.shields.io/crates/v/toad-jni.svg)](https://crates.io/crates/toad-jni)
[![docs.rs](https://docs.rs/toad-jni/badge.svg)](https://docs.rs/toad-jni/latest)
![Maintenance](https://img.shields.io/badge/maintenance-activly--developed-brightgreen.svg)

# toad-jni

High-level wrapper of [`jni`], making Java & Rust FFI easy & fun

### Globals
[`toad_jni::global`](https://docs.rs/toad-jni/latest/toad_jni/global/index.html) offers the option to use a global JVM handle ([`toad_jni::global::jvm()`](https://docs.rs/toad-jni/latest/toad_jni/global/fn.jvm.html) set with [`toad_jni::global::init()`](https://docs.rs/toad-jni/latest/toad_jni/global/fn.init.html)).

Using the JVM global is completely optional, **unless** you plan to use Rust trait impls such as [`IntoIterator`]
on [`toad_jni::java::util::ArrayList`](https://docs.rs/toad-jni/latest/toad_jni/java/util/struct.ArrayList.html).

### Types
All java type signatures can be represented by rust types
that implement the [`toad_jni::java::Type`](https://docs.rs/toad-jni/latest/toad_jni/java/trait.Type.html) trait, which is automatically
implemented for all [`toad_jni::java::Class`](https://docs.rs/toad-jni/latest/toad_jni/java/trait.Class.html)es.

### Classes
Classes are represented in `toad_jni` by implementing 2 traits:
* [`toad_jni::java::Class`](https://docs.rs/toad-jni/latest/toad_jni/java/trait.Class.html)
* [`toad_jni::java::Object`](https://docs.rs/toad-jni/latest/toad_jni/java/trait.Object.html) (see also [`toad_jni::java::object_newtype`](https://docs.rs/toad-jni/latest/toad_jni/java/macro.object_newtype.html))

#### Fields and Methods
There are several high-level lens-style structs for interacting with fields, methods and constructors:
* [`toad_jni::java::Constructor`](https://docs.rs/toad-jni/latest/toad_jni/java/struct.Constructor.html)
* [`toad_jni::java::StaticField`](https://docs.rs/toad-jni/latest/toad_jni/java/struct.StaticField.html)
* [`toad_jni::java::StaticMethod`](https://docs.rs/toad-jni/latest/toad_jni/java/struct.StaticMethod.html)
* [`toad_jni::java::Field`](https://docs.rs/toad-jni/latest/toad_jni/java/struct.Field.html)
* [`toad_jni::java::Method`](https://docs.rs/toad-jni/latest/toad_jni/java/struct.Method.html)

All of these types use [`toad_jni::java::Type`](https://docs.rs/toad-jni/latest/toad_jni/java/trait.Type.html) to transform nice Rust types into the corresponding
JVM type signatures.

For example, the `StaticMethod` representation of [`java.lang.String.format(String, ..Object)`](https://docs.oracle.com/en/java/javase/19/docs/api/java.base/java/lang/String.html#format(java.lang.String,java.lang.Object...))
would be:
```rust
use toad_jni::java::lang::Object;
use toad_jni::java::StaticMethod;

static STRING_FORMAT: StaticMethod<String, fn(String, Vec<Object>) -> String> =
  StaticMethod::new("format");
```

It is recommended that these structs are stored in local `static` variables so that they can cache
the internal JNI IDs of the class and methods, but this is not required.

#### Example
Consider the following java class:
```java
package com.foo.bar;

public class Foo {
  public final static long NUMBER = 123;
  public String bingus = "bingus";

  public Foo() { }

  public static String bar() {
    return "bar";
  }

  public void setBingus(String newBingus) {
    this.bingus = newBingus;
  }
}
```

A Rust API to this class would look like:
```rust
use toad_jni::java;

pub struct Foo(java::lang::Object);

java::object_newtype!(Foo);

impl java::Class for Foo {
  const PATH: &'static str = "com/foo/bar/Foo";
}

impl Foo {
  pub fn new(e: &mut java::Env) -> Self {
    static CTOR: java::Constructor<Foo, fn()> = java::Constructor::new();
    CTOR.invoke(e)
  }

  pub fn number(e: &mut java::Env) -> i64 {
    static NUMBER: java::StaticField<Foo, i64> = java::StaticField::new("NUMBER");
    NUMBER.get(e)
  }

  pub fn bar(e: &mut java::Env) -> String {
    static BAR: java::StaticMethod<Foo, fn() -> String> = java::StaticMethod::new("bar");
    BAR.invoke(e)
  }

  pub fn bingus(&self, e: &mut java::Env) -> String {
    static BINGUS: java::Field<Foo, String> = java::Field::new("bingus");
    BINGUS.get(e, self)
  }

  pub fn set_bingus(&self, e: &mut java::Env, s: String) {
    static SET_BINGUS: java::Method<Foo, fn(String)> = java::Method::new("setBingus");
    SET_BINGUS.invoke(e, self, s)
  }
}
```

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
