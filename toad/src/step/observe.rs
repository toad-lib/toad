use core::fmt::Debug;

use naan::prelude::{Apply, F2Once};
use toad_common::{Array, Stem};
use toad_msg::repeat::QUERY;
use toad_msg::{MessageOptions, OptValue};

use crate::platform::{self, PlatformTypes};

type RequestsSimilar<P> =
  fn(&platform::toad_msg::Message<P>, &platform::toad_msg::Message<P>) -> bool;

/// Default function used by [`Observe::request_similarity`]
///
/// The request should always be a GET, meaning we
/// should not need to consider the method or payload
/// when determining whether requests are similar.
///
/// # How this determines similarity
///  - are the [Type](toad_msg::Message.ty)s equal?
///  - are [Uri-Path](toad_msg::opt::known::no_repeat::HOST) equal?
///  - are [Uri-Query](toad_msg::opt::known::no_repeat::HOST) equal?
///  - are [Accept](toad_msg::opt::known::no_repeat::ACCEPT) equal?
pub fn requests_similar<P>(a: &platform::toad_msg::Message<P>,
                           b: &platform::toad_msg::Message<P>)
                           -> bool
  where P: PlatformTypes
{
  fn values_eq<Vs, V>(a: &Vs, b: &Vs) -> bool
    where V: Array<Item = u8>,
          Vs: Array<Item = OptValue<V>>
  {
    a.iter().eq(b.iter())
  }

  let (a_query, b_query) = (a.get(QUERY), b.get(QUERY));
  let neither_has_query = a_query.is_none() && b_query.is_none();
  let query_eq = || {
    neither_has_query
    || Some(values_eq.curry()).apply(a_query)
                              .apply(b_query)
                              .unwrap_or(false)
  };

  a.ty == b.ty && a.accept() == b.accept() && a.path().ok() == b.path().ok() && query_eq()
}

/// See [the module documentation](self)
pub struct Observe<P, S, B>
  where P: PlatformTypes
{
  inner: S,
  subs: Stem<B>,
  request_similarity_invoked: bool,
  similar_fn: RequestsSimilar<P>,
}

impl<P, S, B> Debug for Observe<P, S, B>
  where P: PlatformTypes,
        S: Debug,
        B: Debug
{
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_struct("Observe")
     .field("inner", &self.inner)
     .field("subs", &self.subs)
     .field("request_similarity_invoked", &self.request_similarity_invoked)
     .field("similar_fn", &"<fn pointer>")
     .finish()
  }
}

impl<P, S, B> Default for Observe<P, S, B>
  where P: PlatformTypes,
        S: Default,
        B: Default
{
  fn default() -> Self {
    Self { inner: Default::default(),
           subs: Default::default(),
           similar_fn: requests_similar::<P>,
           request_similarity_invoked: false }
  }
}

impl<P, S, B> Observe<P, S, B> where P: PlatformTypes
{
  /// Change the function used to test request similarity
  ///
  /// When this step is notified of a new version of a resource,
  /// it will send copies of all subscribers' original GET requests
  /// to your server, then send the responses to the subscribers as events.
  ///
  /// This function allows the Observe step to deduplicate these requests,
  /// allowing your server to notify multiple subscribers that have similar subscriptions
  /// by handling a single request.
  ///
  /// By default this uses [`requests_similar`].
  ///
  /// # Panics
  /// Panics if invoked more than once. Because the usecase for this method is
  /// a one-time "don't use the default, use my function instead," this unsafely
  /// mutates `self` from the immutable method receiver to keep runtime cost low.
  ///
  /// Do not call this method more than once.
  ///
  /// Do not call this method after the server has started.
  ///
  /// Do not pass Go, and do not collect $200.
  pub fn request_similarity(&self, f: RequestsSimilar<P>) {
    // SAFETY:
    // I am comfortable shunting this unsafety to users - this function
    // can only ever be called once and is clearly intended to be called before
    // bootstrapping (i.e. before anyone else can even access this struct)
    #[allow(unsafe_code)]
    unsafe {
      if self.request_similarity_invoked {
        panic!("Observe::request_similarity may only be invoked once");
      }

      let me = (self as *const Self as *mut Self).as_mut().unwrap();
      me.request_similarity_invoked = true;
      me.similar_fn = f;
    }
  }
}

#[cfg(test)]
mod tests {
  use ::toad_msg::{Code, ContentFormat, Id, Token, Type};
  use platform::toad_msg;

  use super::*;
  use crate::test;

  type Message = toad_msg::Message<test::Platform>;

  #[test]
  pub fn requests_similar_() {
    fn req<F>(stuff: F) -> Message
      where F: FnOnce(&mut Message)
    {
      let mut req = toad_msg::Message::<test::Platform>::new(Type::Con,
                                                             Code::GET,
                                                             Id(1),
                                                             Token(Default::default()));
      stuff(&mut req);
      req
    }

    assert!(!requests_similar::<test::Platform>(&req(|r| {
                                                  r.set_path("a/b/c").ok();
                                                }),
                                                &req(|_| {})));
    assert!(requests_similar::<test::Platform>(&req(|r| {
                                                 r.set_path("a/b/c").ok();
                                               }),
                                               &req(|r| {
                                                 r.set_path("a/b/c").ok();
                                               })));
    assert!(!requests_similar::<test::Platform>(&req(|r| {
                                                  r.set_path("a/b/c").ok();
                                                  r.add_query("filter[temp](less_than)=123").ok();
                                                }),
                                                &req(|r| {
                                                  r.set_path("a/b/c").ok();
                                                })));
    assert!(requests_similar::<test::Platform>(&req(|r| {
                                                 r.set_path("a/b/c").ok();
                                                 r.add_query("filter[temp](less_than)=123").ok();
                                               }),
                                               &req(|r| {
                                                 r.set_path("a/b/c").ok();
                                                 r.add_query("filter[temp](less_than)=123").ok();
                                               })));
    assert!(!requests_similar::<test::Platform>(&req(|r| {
                                                  r.set_path("a/b/c").ok();
                                                  r.add_query("filter[temp](less_than)=123").ok();
                                                  r.set_accept(ContentFormat::Json).ok();
                                                }),
                                                &req(|r| {
                                                  r.set_path("a/b/c").ok();
                                                  r.add_query("filter[temp](less_than)=123").ok();
                                                  r.set_accept(ContentFormat::Text).ok();
                                                })));
    assert!(requests_similar::<test::Platform>(&req(|r| {
                                                 r.set_path("a/b/c").ok();
                                                 r.add_query("filter[temp](less_than)=123").ok();
                                                 r.set_accept(ContentFormat::Json).ok();
                                               }),
                                               &req(|r| {
                                                 r.set_path("a/b/c").ok();
                                                 r.add_query("filter[temp](less_than)=123").ok();
                                                 r.set_accept(ContentFormat::Json).ok();
                                               })));
  }

  #[test]
  fn request_similarity_first_invocation_should_not_panic() {
    let o = Observe::<test::Platform, (), ()>::default();
    o.request_similarity(requests_similar::<test::Platform>);
  }

  #[test]
  #[should_panic]
  fn request_similarity_second_invocation_should_panic() {
    let o = Observe::<test::Platform, (), ()>::default();
    o.request_similarity(requests_similar::<test::Platform>);
    o.request_similarity(requests_similar::<test::Platform>);
  }
}
