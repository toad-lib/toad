/// Content-Format values
pub mod content_format;
pub use content_format::*;

macro_rules! opt {
  (rfc7252($section:literal) $name:ident = $n:literal) => {
    #[doc = ::toad_macros::rfc_7252_doc!($section)]
    #[allow(clippy::zero_prefixed_literal)]
    pub const $name: crate::OptNumber = crate::OptNumber($n);
  };
  (#[doc = $doc:expr] $name:ident = $n:literal) => {
    #[doc = $doc]
    #[allow(clippy::zero_prefixed_literal)]
    pub const $name: crate::OptNumber = crate::OptNumber($n);
  };
}

pub(crate) use opt;

/// Non-repeatable options
pub mod no_repeat {
  use super::opt;

  opt!(rfc7252("5.10.1") HOST = 3);
  opt!(rfc7252("5.10.8.2") IF_NONE_MATCH = 5);
  opt!(#[doc = "See [`HOST`]"]
       PORT = 7);
  opt!(#[doc = "See [`HOST`]"]
       PATH = 11);
  opt!(rfc7252("5.10.3") CONTENT_FORMAT = 12);
  opt!(rfc7252("5.10.5") MAX_AGE = 14);
  opt!(rfc7252("5.10.4") ACCEPT = 17);
  opt!(rfc7252("5.10.2") PROXY_URI = 35);
  opt!(#[doc = "See [`PROXY_URI`]"]
       PROXY_SCHEME = 39);
  opt!(rfc7252("5.10.9") SIZE1 = 60);
}

/// Repeatable options
pub mod repeat {
  use super::opt;

  opt!(rfc7252("5.10.8.1") IF_MATCH = 1);
  opt!(rfc7252("5.10.7") LOCATION_PATH = 8);
  opt!(#[doc = "See [`super::no_repeat::HOST`]"]
       QUERY = 11);
  opt!(#[doc = "See [`LOCATION_PATH`]"]
       LOCATION_QUERY = 20);
  opt!(#[doc = concat!(
                toad_macros::rfc_7252_doc!("5.10.6"),
                "\n<details><summary>ETag as a Request Option</summary>\n\n",
                toad_macros::rfc_7252_doc!("5.10.6.2"),
                "\n</details><details><summary>ETag as a Response Option</summary>\n\n",
                toad_macros::rfc_7252_doc!("5.10.6.1"),
                "</details>"
      )]
       ETAG = 4);
}
