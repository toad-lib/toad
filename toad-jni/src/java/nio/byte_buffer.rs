use std::io::Write;
use std::ops::RangeBounds;
use std::slice::SliceIndex;

use crate::java::{self, NoUpcast, Object, ResultExt, Signature};

/// `java.nio.Buffer`
pub struct Buffer(java::lang::Object);
java::object_newtype!(Buffer);
impl java::Class for Buffer {
  const PATH: &'static str = "java/nio/Buffer";
}

/// `java.nio.ByteBuffer`
pub struct ByteBuffer(java::lang::Object);
java::object_newtype!(ByteBuffer);
impl java::Class for ByteBuffer {
  const PATH: &'static str = "java/nio/ByteBuffer";
}

impl ByteBuffer {
  /// `java.nio.ByteBuffer.wrap(byte[])`
  pub fn new(e: &mut java::Env, bytes: impl IntoIterator<Item = u8>) -> Self {
    static WRAP: java::StaticMethod<ByteBuffer, fn(Vec<i8>) -> ByteBuffer> =
      java::StaticMethod::new("wrap");
    WRAP.invoke(e,
                bytes.into_iter()
                     .map(|u| i8::from_be_bytes(u.to_be_bytes()))
                     .collect())
  }

  /// Upcast `Buffer` to `ByteBuffer`
  pub fn from_buf(buf: Buffer) -> Self {
    Self(buf.0)
  }

  /// Downcast self to `Buffer` (superclass of `ByteBuffer`)
  pub fn as_buf(&self, e: &mut java::Env) -> Buffer {
    self.0.downcast_ref(e).upcast_to::<Buffer>(e)
  }

  /// `java.nio.Buffer.capacity()`
  pub fn len(&self, e: &mut java::Env) -> u32 {
    static CAPACITY: java::Method<Buffer, fn() -> i32> = java::Method::new("capacity");
    let buf = self.as_buf(e);
    CAPACITY.invoke(e, &buf) as u32
  }

  /// `java.nio.ByteBuffer.rewind()`
  pub fn rewind(&self, e: &mut java::Env) {
    static REWIND: java::Method<ByteBuffer, fn() -> ByteBuffer> = java::Method::new("rewind");
    REWIND.invoke(e, self);
  }

  /// `java.nio.Buffer.position()`
  pub fn position(&self, e: &mut java::Env) -> u32 {
    static POSITION: java::Method<Buffer, fn() -> i32> = java::Method::new("position");
    let buf = self.as_buf(e);
    POSITION.invoke(e, &buf) as u32
  }

  /// Write the contents of this byte buffer to a rust buffer
  pub fn write_to(&self, e: &mut java::Env, start: usize, end: usize, mut buf: &mut [u8]) -> usize {
    let vec = self.to_vec(e);
    buf.write(&vec[start..=end]).unwrap()
  }

  /// Extract the contents of this byte buffer to a vec of bytes
  pub fn to_vec(&self, e: &mut java::Env) -> Vec<u8> {
    let len = self.len(e);

    let mut bytes = Vec::<u8>::new();
    bytes.resize(len as usize, 0);

    let bytes_u8_mut = bytes.as_mut_slice();

    // SAFETY:
    // transmute [i8] to [u8] is always safe
    let bytes_i8_mut = unsafe { core::mem::transmute::<&mut [u8], &mut [i8]>(bytes_u8_mut) };

    let arr = e.new_byte_array(len as i32).unwrap();

    e.call_method(self.0.as_local(),
                  "get",
                  Signature::of::<fn(Vec<i8>) -> ByteBuffer>(),
                  &[(&arr).into()])
     .unwrap_java(e);
    e.get_byte_array_region(&arr, 0, bytes_i8_mut).unwrap();
    bytes
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn roundtrip() {
    let mut e = crate::test::init();
    let e = &mut e;

    let foobar = "foobar is a standard test string that is used to generate dummy data such as this byte array.".as_bytes().to_vec();
    let buf = ByteBuffer::new(e, foobar.as_slice().iter().copied());
    let foobar_out = buf.to_vec(e);

    assert_eq!(foobar, foobar_out);
  }

  #[test]
  fn zero() {
    let mut e = crate::test::init();
    let e = &mut e;

    let buf = ByteBuffer::new(e, [0u8; 0]);
    let foobar_out = buf.to_vec(e);

    assert_eq!(foobar_out, [0u8; 0]);
  }
}
