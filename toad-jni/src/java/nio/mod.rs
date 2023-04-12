use crate::java;

/// `java.nio.channels`
pub mod channels;

mod byte_buffer;
pub use byte_buffer::ByteBuffer;

/// `java.nio.channels.SelectableChannel`
pub struct SelectableChannel(java::lang::Object);
java::object_newtype!(SelectableChannel);
impl java::Class for SelectableChannel {
  const PATH: &'static str = "java/nio/channels/SelectableChannel";
}
