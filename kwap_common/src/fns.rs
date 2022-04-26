/// Returns a function that discards its argument and always returns `r`.
///
/// ```
/// use kwap_common::prelude::*;
///
/// fn try_get_string() -> Result<String, std::io::Error> {
///   # Ok("".into())
/// }
///
/// fn do_stuff() -> Result<String, std::io::Error> {
///   try_get_string().map(const_("it worked!".into())) // equivalent to:
/// }
/// ```
pub fn const_<T, R>(r: R) -> impl FnOnce(T) -> R {
  |_| r
}

/// A function that discards its argument and always returns unit `()`
///
/// ```
/// use kwap_common::prelude::*;
///
/// fn try_get_string() -> Result<String, std::io::Error> {
///   # Ok("".into())
/// }
///
/// fn do_stuff() -> Result<(), std::io::Error> {
///   try_get_string().map(ignore) // equivalent to:
///                   .map(|_| ())
/// }
/// ```
pub fn ignore<T>(_: T) {
  ()
}
