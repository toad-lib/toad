/// Returns a function that discards its argument and always returns `r`.
///
/// ```compile_fail
/// use kwap_util::const_;
///
/// fn try_get_string() -> Result<String, std::io::Error> {
///   # Ok("".into())
/// }
///
/// fn do_stuff() -> Result<String, std::io::Error> {
///   try_get_string()
///     .map(const_("it worked!".into())) // equivalent to:
/// }
/// ```
#[allow(unreachable_pub)]
pub fn const_<T, R>(r: R) -> impl FnOnce(T) -> R {
  |_| r
}

/// A function that discards its argument and always returns unit `()`
///
/// ```compile_fail
/// use kwap_util::ignore;
///
/// fn try_get_string() -> Result<String, std::io::Error> {
///   # Ok("".into())
/// }
///
/// fn do_stuff() -> Result<(), std::io::Error> {
///   try_get_string()
///     .map(ignore) // equivalent to:
///     .map(|_| ())
/// }
/// ```
#[allow(unreachable_pub)]
pub fn ignore<T>(_: T) {
  ()
}
