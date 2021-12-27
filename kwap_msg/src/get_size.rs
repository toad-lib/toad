/// Get the runtime size (in bytes) of a struct
pub trait GetSize {
  /// Get the runtime size (in bytes) of a struct
  fn get_size(&self) -> usize;
}
