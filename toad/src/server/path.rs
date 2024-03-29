use core::fmt::Write;

use crate::platform::PlatformTypes;
use crate::server::ap::state::{ApState, Combine, Hydrated};
use crate::server::ap::{Ap, Hydrate};
use crate::todo::String;

/// Manipulate & match against path segments
pub mod segment {
  use super::*;

  /// Get the next path segment
  ///
  /// ```
  /// use toad::net::Addrd;
  /// use toad::req::Req;
  /// use toad::server::ap::{state, Ap, Hydrate};
  /// use toad::server::path;
  /// use toad::std::{dtls, PlatformTypes as Std};
  ///
  /// # let addr = || {
  /// #   use no_std_net::*;
  /// #   SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(192, 168, 0, 1), 8080))
  /// # };
  /// let addr = addr(); // 192.168.0.1:8080
  /// let req = Req::<Std<dtls::Y>>::get("a/b/c");
  /// let ap: Ap<_, Std<dtls::Y>, (), ()> =
  ///   Ap::ok_hydrated((), Hydrate::from_request(Addrd(req, addr)));
  ///
  /// ap.pipe(path::segment::next(|_, a| Ap::ok(assert_eq!(a.unwrap(), "a"))))
  ///   .pipe(path::segment::next(|_, b| Ap::ok(assert_eq!(b.unwrap(), "b"))))
  ///   .pipe(path::segment::next(|_, c| Ap::ok(assert_eq!(c.unwrap(), "c"))));
  /// ```
  pub fn next<T, SOut, R, F, P, E>(
    f: F)
    -> impl FnOnce(Ap<Hydrated, P, T, E>) -> Ap<<Hydrated as Combine<SOut>>::Out, P, R, E>
    where P: PlatformTypes,
          F: for<'a> FnOnce(T, Option<&'a str>) -> Ap<SOut, P, R, E>,
          E: core::fmt::Debug,
          SOut: ApState,
          Hydrated: Combine<SOut>
  {
    |ap| match ap.try_unwrap_ok_hydrated() {
      | Ok((t, Hydrate { path, path_ix, req })) => {
        if path_ix >= path.len() {
          Ap::ok_hydrated(t, Hydrate { req, path_ix, path }).bind(|t| f(t, None))
        } else {
          let seg_str = path.get(path_ix)
                            .map(|seg| core::str::from_utf8(&seg.0).unwrap())
                            .unwrap_or("");

          let ap_r = f(t, Some(seg_str));

          Ap::ok_hydrated((),
                          Hydrate { req,
                                    path_ix: path_ix + 1,
                                    path }).bind(|_| ap_r)
        }
      },
      | Err(other) => other.bind(|_| unreachable!()).coerce_state(),
    }
  }

  /// Helper functions for adding filters against path segments
  pub mod check {
    use super::*;

    /// Reject the request if the next path segment does not match a predicate `F: FnOnce(&str) -> bool`
    ///
    /// ```
    /// use toad::net::Addrd;
    /// use toad::req::Req;
    /// use toad::server::ap::{state, Ap, Hydrate};
    /// use toad::server::path;
    /// use toad::std::{dtls, PlatformTypes as Std};
    ///
    /// # let addr = || {
    /// #   use no_std_net::*;
    /// #   SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(192, 168, 0, 1), 8080))
    /// # };
    /// let addr = addr(); // 192.168.0.1:8080
    ///
    /// let fruit_request = Req::<Std<dtls::Y>>::get("fruit/banana");
    /// let fruit_ap: Ap<_, Std<dtls::Y>, (), ()> =
    ///   Ap::ok_hydrated((), Hydrate::from_request(Addrd(fruit_request, addr)));
    /// let fruit_filtered = fruit_ap.pipe(path::segment::check::next_is(|s| s == "fruit"));
    /// assert!(fruit_filtered.is_ok());
    /// assert!(fruit_filtered.pipe(path::segment::check::next_is(|s| s == "meat"))
    ///                       .is_rejected());
    ///
    /// let meat_request = Req::<Std<dtls::Y>>::get("meat/steak");
    /// let meat_ap: Ap<_, Std<dtls::Y>, (), ()> =
    ///   Ap::ok_hydrated((), Hydrate::from_request(Addrd(meat_request, addr)));
    /// let meat_filtered = meat_ap.pipe(path::segment::check::next_is(|s| s == "meat"));
    /// assert!(meat_filtered.is_ok());
    /// assert!(meat_filtered.pipe(path::segment::check::next_is(|s| s == "fruit"))
    ///                      .is_rejected());
    /// ```
    pub fn next_is<F, P, T, E>(f: F) -> impl FnOnce(Ap<Hydrated, P, T, E>) -> Ap<Hydrated, P, T, E>
      where P: PlatformTypes,
            E: core::fmt::Debug,
            F: for<'a> FnOnce(&'a str) -> bool
    {
      next(move |t, a| match a {
        | Some(a) if f(a) => Ap::ok(t),
        | _ => Ap::reject().pretend_unhydrated(),
      })
    }

    /// Reject the request if the next path segment does not equal `path`
    pub fn next_equals<A, P, T, E>(path: A)
                                   -> impl FnOnce(Ap<Hydrated, P, T, E>) -> Ap<Hydrated, P, T, E>
      where P: PlatformTypes,
            E: core::fmt::Debug,
            A: AsRef<str> + 'static
    {
      next_is(move |s| s == path.as_ref())
    }
  }

  /// Route parameter extraction
  pub mod param {
    use super::*;

    /// Consume the next path segment as an integer
    ///
    /// If the segment fails to be parsed with [`u32::from_str_radix`],
    /// the request will be rejected.
    ///
    /// ```
    /// use toad::net::Addrd;
    /// use toad::req::Req;
    /// use toad::server::ap::{state, Ap, Hydrate};
    /// use toad::server::path;
    /// use toad::std::{dtls, PlatformTypes as Std};
    ///
    /// # let addr = || {
    /// #   use no_std_net::*;
    /// #   SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(192, 168, 0, 1), 8080))
    /// # };
    /// let addr = addr(); // 192.168.0.1:8080
    ///
    /// let num_request = Req::<Std<dtls::Y>>::get("1234");
    /// let num_ap: Ap<_, Std<dtls::Y>, (), ()> =
    ///   Ap::ok_hydrated((), Hydrate::from_request(Addrd(num_request, addr)));
    ///
    /// assert_eq!(num_ap.pipe(path::segment::param::u32)
    ///                  .try_unwrap_ok()
    ///                  .unwrap(),
    ///            ((), 1234));
    ///
    /// let other_request = Req::<Std<dtls::Y>>::get("foobar");
    /// let other_ap: Ap<_, Std<dtls::Y>, (), ()> =
    ///   Ap::ok_hydrated((), Hydrate::from_request(Addrd(other_request, addr)));
    /// assert!(other_ap.pipe(path::segment::param::u32).is_rejected());
    /// ```
    pub fn u32<P, T, E>(ap: Ap<Hydrated, P, T, E>) -> Ap<Hydrated, P, (T, u32), E>
      where P: PlatformTypes,
            E: core::fmt::Debug
    {
      let parse = |s: &str| u32::from_str_radix(s, 10);

      next(|t, s| {
        s.map(Ap::ok)
         .unwrap_or_else(|| Ap::reject().pretend_unhydrated())
         .map(parse)
         .bind(Ap::from_result)
         .reject_on_err()
         .map(|u| (t, u))
      })(ap)
    }
  }
}

/// Get the rest of the request path, skipping any
/// consumed [`segment`]s.
pub fn rest<T, SOut, R, F, P, E>(
  f: F)
  -> impl FnOnce(Ap<Hydrated, P, T, E>) -> Ap<<Hydrated as Combine<SOut>>::Out, P, R, E>
  where P: PlatformTypes,
        F: for<'a> FnOnce(T, &'a str) -> Ap<SOut, P, R, E>,
        E: core::fmt::Debug,
        SOut: ApState,
        Hydrated: Combine<SOut>
{
  |ap| match ap.try_unwrap_ok_hydrated() {
    | Ok((t, Hydrate { path, req, path_ix })) => {
      let mut s = match path.get(path_ix..) {
        | Some(segs) => segs.iter().fold(String::<1000>::default(), |mut s, seg| {
                                     if let Ok(seg) = core::str::from_utf8(seg.as_bytes()) {
                                       write!(&mut s, "{seg}/").ok();
                                     }
                                     s
                                   }),
        | None => String::<1000>::default(),
      };

      s.as_writable().pop();

      let ap_r = f(t, s.as_str());
      Ap::ok_hydrated((),
                      Hydrate { req,
                                path_ix: path.len(),
                                path }).bind(|_| ap_r)
    },
    | Err(other) => other.bind(|_| unreachable!()).coerce_state(),
  }
}

/// Helper functions for adding filters against whole paths
pub mod check {
  use super::*;

  /// Reject the request if the rest of the path does not match a predicate `F: FnOnce(&str) -> bool`
  pub fn rest_is<F, P, T, E>(f: F) -> impl FnOnce(Ap<Hydrated, P, T, E>) -> Ap<Hydrated, P, T, E>
    where P: PlatformTypes,
          E: core::fmt::Debug,
          F: for<'a> FnOnce(&'a str) -> bool
  {
    rest(move |t, a| match a {
      | a if f(a) => Ap::ok(t),
      | _ => Ap::reject().pretend_unhydrated(),
    })
  }

  /// Reject the request if the rest of the path does not equal `path`
  pub fn rest_equals<A, P, T, E>(path: A)
                                 -> impl FnOnce(Ap<Hydrated, P, T, E>) -> Ap<Hydrated, P, T, E>
    where P: PlatformTypes,
          E: core::fmt::Debug,
          A: AsRef<str> + 'static
  {
    rest_is(move |a| a == path.as_ref())
  }

  /// Reject the request if the rest of the path does not end with `path`
  pub fn ends_with<A, T, P, E>(path: A)
                               -> impl FnOnce(Ap<Hydrated, P, T, E>) -> Ap<Hydrated, P, T, E>
    where P: PlatformTypes,
          A: AsRef<str> + 'static,
          E: core::fmt::Debug
  {
    rest_is(move |a| a.ends_with(path.as_ref()))
  }
}

#[cfg(test)]
mod tests {
  use toad_msg::MessageOptions;

  use super::*;
  use crate::req::Req;
  use crate::server::path;

  type Ap<S, T, E> = super::Ap<S, crate::test::Platform, T, E>;

  #[test]
  fn rest() {
    let req = || {
      let mut r = crate::test::msg!(CON GET x.x.x.x:1111).map(Req::from);
      r.as_mut().msg_mut().set_path("foo/bar").unwrap();
      r
    };

    let ap = Ap::<_, (), ()>::ok_hydrated((), Hydrate::from_request(req()));
    let ap_path = ap.pipe(path::rest(|(), s| Ap::ok(s.to_string())));

    assert_eq!(ap_path.clone().try_unwrap_ok(), Ok("foo/bar".to_string()));

    assert_eq!(ap_path.pipe(path::rest(|_, s| Ap::ok(s.to_string())))
                      .try_unwrap_ok(),
               Ok("".to_string()));
  }

  #[test]
  fn rest_is() {
    let req = |p| {
      let mut r = crate::test::msg!(CON GET x.x.x.x:1111).map(Req::from);
      r.as_mut().msg_mut().set_path(p).unwrap();
      r
    };

    let pass = Hydrate::from_request(req("foo/bar"));
    let fail = Hydrate::from_request(req("foot/bart"));

    let check = |hy| {
      Ap::<_, (), ()>::ok_hydrated((), hy).pipe(path::check::rest_is(|p| p == "foo/bar"))
                                          .try_unwrap_ok()
    };

    assert!(matches!(check(pass), Ok(_)));
    assert!(matches!(check(fail), Err(_)));
  }

  #[test]
  fn rest_equals() {
    let req = |p| {
      let mut r = crate::test::msg!(CON GET x.x.x.x:1111).map(Req::from);
      r.as_mut().msg_mut().set_path(p).unwrap();
      r
    };

    let pass = Hydrate::from_request(req("foo/bar"));
    let fail = Hydrate::from_request(req("foot/bart"));

    let check = |hy| {
      Ap::<_, (), ()>::ok_hydrated((), hy).pipe(path::check::rest_equals("foo/bar"))
                                          .try_unwrap_ok()
    };

    assert!(matches!(check(pass), Ok(_)));
    assert!(matches!(check(fail), Err(_)));
  }

  #[test]
  fn ends_with() {
    let req = |p| {
      let mut r = crate::test::msg!(CON GET x.x.x.x:1111).map(Req::from);
      r.as_mut().msg_mut().set_path(p).unwrap();
      r
    };

    let pass = Hydrate::from_request(req("foo/bar"));
    let fail = Hydrate::from_request(req("foot/bart"));

    let check = |hy| {
      Ap::<_, (), ()>::ok_hydrated((), hy).pipe(path::check::ends_with("bar"))
                                          .try_unwrap_ok()
    };

    assert!(matches!(check(pass), Ok(_)));
    assert!(matches!(check(fail), Err(_)));
  }

  #[test]
  fn next_segment() {
    let req = |p| {
      let mut r = crate::test::msg!(CON GET x.x.x.x:1111).map(Req::from);
      r.as_mut().msg_mut().set_path(p).unwrap();
      r
    };

    let hy = Hydrate::from_request(req("foot/bart"));

    let ap = Ap::<_, (), ()>::ok_hydrated((), hy).pipe(path::segment::next(|_, s| {
                                                         Ap::ok(s.unwrap().to_string())
                                                       }));

    assert_eq!(ap.clone().try_unwrap_ok(), Ok("foot".to_string()));

    assert_eq!(ap.pipe(path::segment::next(|_, s| Ap::ok(s.unwrap().to_string())))
                 .try_unwrap_ok(),
               Ok("bart".to_string()));
  }

  #[test]
  fn segment_param() {
    let req = |p| {
      let mut r = crate::test::msg!(CON GET x.x.x.x:1111).map(Req::from);
      r.as_mut().msg_mut().set_path(p).unwrap();
      r
    };

    let hy = Hydrate::from_request(req("users/123"));

    let ap = Ap::<_, (), ()>::ok_hydrated((), hy).pipe(path::segment::check::next_equals("users"))
                                                 .pipe(path::segment::param::u32)
                                                 .map(|(_, u)| u);

    assert_eq!(ap.clone().try_unwrap_ok(), Ok(123));
  }
}
