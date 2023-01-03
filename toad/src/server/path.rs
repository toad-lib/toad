use toad_common::Cursor;

use crate::platform::PlatformTypes;
use crate::server::ap::state::{ApState, Combine, Hydrated};
use crate::server::ap::{Ap, Hydrate};

macro_rules! was_not_ok_hy {
    ($other:expr) => {
        unreachable!("State type argument was `Hydrated`, so I expected the runtime value to be Ap(ApInner::OkHydrated {{ .. }}), but got {:?}", $other.map(|_| ()).0)
    };
  }

pub mod segment {
  use super::*;

  pub fn next<T, SOut, R, F, P, E>(
    f: F)
    -> impl FnOnce(Ap<Hydrated, P, T, E>) -> Ap<<SOut as Combine<Hydrated>>::Out, P, R, E>
    where P: PlatformTypes,
          F: for<'a> FnOnce(T, Option<&'a str>) -> Ap<SOut, P, R, E>,
          E: core::fmt::Debug,
          SOut: ApState
  {
    |ap| {
      match ap.try_unwrap_ok_hydrated() {
        | Ok((t, Hydrate { mut path, req })) => {
          if path.is_exhausted() {
            f(t, None).bind(|r| Ap::ok_hydrated(r, Hydrate { req, path }))
          } else {
            let seg = Cursor::take_while(&mut path, |b: u8| (b as char) != '/');
            let seg_str = core::str::from_utf8(seg).unwrap();

            let ap_r = f(t, Some(seg_str));

            // skip the slash
            drop(seg_str);
            Cursor::skip(&mut path, 1);

            ap_r.bind(|r| Ap::ok_hydrated(r, Hydrate { req, path }))
          }
        },
        | Err(other) => was_not_ok_hy!(other),
      }
    }
  }

  pub mod check {
    use super::*;
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

    pub fn next_equals<A, P, T, E>(path: A)
                                   -> impl FnOnce(Ap<Hydrated, P, T, E>) -> Ap<Hydrated, P, T, E>
      where P: PlatformTypes,
            E: core::fmt::Debug,
            A: AsRef<str> + 'static
    {
      next_is(move |s| s == path.as_ref())
    }
  }

  pub mod param {
    use super::*;
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

pub fn rest<T, SOut, R, F, P, E>(
  f: F)
  -> impl FnOnce(Ap<Hydrated, P, T, E>) -> Ap<<SOut as Combine<Hydrated>>::Out, P, R, E>
  where P: PlatformTypes,
        F: for<'a> FnOnce(T, &'a str) -> Ap<SOut, P, R, E>,
        E: core::fmt::Debug,
        SOut: ApState
{
  |ap| match ap.try_unwrap_ok_hydrated() {
    | Ok((t, Hydrate { mut path, req })) => {
      let seg = Cursor::take_until_end(&mut path);
      let seg_str = core::str::from_utf8(seg).unwrap();

      f(t, seg_str).bind(|r| Ap::ok_hydrated(r, Hydrate { req, path }))
    },
    | Err(other) => was_not_ok_hy!(other),
  }
}

pub mod check {
  use super::*;

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

  pub fn rest_equals<A, P, T, E>(path: A)
                                 -> impl FnOnce(Ap<Hydrated, P, T, E>) -> Ap<Hydrated, P, T, E>
    where P: PlatformTypes,
          E: core::fmt::Debug,
          A: AsRef<str> + 'static
  {
    rest_is(move |a| a == path.as_ref())
  }

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
  use super::*;
  use crate::req::Req;
  use crate::server::path;

  type Ap<S, T, E> = super::Ap<S, crate::test::Platform, T, E>;

  #[test]
  fn rest() {
    let req = || crate::test::msg!(CON GET x.x.x.x:1111).map(Req::from);
    let foobar = || Cursor::new("foo/bar".into());
    let ap = Ap::<_, (), ()>::ok_hydrated((),
                                          Hydrate { req: req(),
                                                    path: foobar() });
    let ap_path = ap.pipe(path::rest(|(), s| Ap::ok(s.to_string())));

    assert_eq!(ap_path.clone().try_unwrap_ok(), Ok("foo/bar".to_string()));

    assert_eq!(ap_path.pipe(path::rest(|_, s| Ap::ok(s.to_string())))
                      .try_unwrap_ok(),
               Ok("".to_string()));
  }

  #[test]
  fn rest_is() {
    let req = || crate::test::msg!(CON GET x.x.x.x:1111).map(Req::from);

    let pass = Hydrate { req: req(),
                         path: Cursor::new("foo/bar".into()) };

    let fail = Hydrate { req: req(),
                         path: Cursor::new("foot/bart".into()) };

    let check = |hy| {
      Ap::<_, (), ()>::ok_hydrated((), hy).pipe(path::check::rest_is(|p| p == "foo/bar"))
                                          .try_unwrap_ok()
    };

    assert!(matches!(check(pass), Ok(_)));
    assert!(matches!(check(fail), Err(_)));
  }

  #[test]
  fn rest_equals() {
    let req = || crate::test::msg!(CON GET x.x.x.x:1111).map(Req::from);

    let pass = Hydrate { req: req(),
                         path: Cursor::new("foo/bar".into()) };

    let fail = Hydrate { req: req(),
                         path: Cursor::new("foot/bart".into()) };

    let check = |hy| {
      Ap::<_, (), ()>::ok_hydrated((), hy).pipe(path::check::rest_equals("foo/bar"))
                                          .try_unwrap_ok()
    };

    assert!(matches!(check(pass), Ok(_)));
    assert!(matches!(check(fail), Err(_)));
  }

  #[test]
  fn ends_with() {
    let req = || crate::test::msg!(CON GET x.x.x.x:1111).map(Req::from);

    let pass = Hydrate { req: req(),
                         path: Cursor::new("foo/bar".into()) };

    let fail = Hydrate { req: req(),
                         path: Cursor::new("foot/bart".into()) };

    let check = |hy| {
      Ap::<_, (), ()>::ok_hydrated((), hy).pipe(path::check::ends_with("bar"))
                                          .try_unwrap_ok()
    };

    assert!(matches!(check(pass), Ok(_)));
    assert!(matches!(check(fail), Err(_)));
  }

  #[test]
  fn next_segment() {
    let hy = Hydrate { req: crate::test::msg!(CON GET x.x.x.x:1111).map(Req::from),
                       path: Cursor::new("foot/bart".into()) };

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
    let hy = Hydrate { req: crate::test::msg!(CON GET x.x.x.x:1111).map(Req::from),
                       path: Cursor::new("users/123".into()) };

    let ap = Ap::<_, (), ()>::ok_hydrated((), hy).pipe(path::segment::check::next_equals("users"))
                                                 .pipe(path::segment::param::u32)
                                                 .map(|(_, u)| u);

    assert_eq!(ap.clone().try_unwrap_ok(), Ok(123));
  }
}
