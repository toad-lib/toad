use toad_array::Array;

/// Content-Format
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub enum ContentFormat {
  /// `text/plain; charset=utf-8`
  Text,
  /// `application/link-format`
  LinkFormat,
  /// `application/xml`
  Xml,
  /// `application/octet-stream`
  OctetStream,
  /// `application/exi`
  Exi,
  /// `application/json`
  Json,
  /// Another content format
  Other(u16),
}

impl ContentFormat {
  /// Convert this content format to the CoAP byte value
  pub fn bytes(&self) -> [u8; 2] {
    u16::from(self).to_be_bytes()
  }
}

impl<'a> From<&'a ContentFormat> for u16 {
  fn from(f: &'a ContentFormat) -> Self {
    use ContentFormat::*;
    match *f {
      | Text => 0,
      | LinkFormat => 40,
      | Xml => 41,
      | OctetStream => 42,
      | Exi => 47,
      | Json => 50,
      | Other(n) => n,
    }
  }
}

impl ToCoapValue for ContentFormat {
  fn to_coap_value<T: Array<Item = u8>>(self) -> T {
    self.bytes().into_iter().collect()
  }
}

/// Something that can be stored in a CoAP Option.
///
/// These include:
/// - strings (str and String)
/// - empty (`()`)
/// - unsigned integers (`u8`, `u16`, `u32`, `u64`)
/// - bytes (anything that impls [`toad_common::Array`])
pub trait ToCoapValue {
  /// Convert the value
  fn to_coap_value<T: Array<Item = u8>>(self) -> T;
}

impl<'a> ToCoapValue for &'a str {
  fn to_coap_value<T: Array<Item = u8>>(self) -> T {
    self.bytes().to_coap_value()
  }
}

impl ToCoapValue for core::str::Bytes<'_> {
  fn to_coap_value<T: Array<Item = u8>>(self) -> T {
    self.collect()
  }
}

impl<A: tinyvec::Array<Item = u8>> ToCoapValue for tinyvec::ArrayVec<A> {
  fn to_coap_value<T: Array<Item = u8>>(self) -> T {
    self.into_iter().collect()
  }
}

impl ToCoapValue for &[u8] {
  fn to_coap_value<T: Array<Item = u8>>(self) -> T {
    self.iter().copied().collect()
  }
}

impl ToCoapValue for u8 {
  fn to_coap_value<T: Array<Item = u8>>(self) -> T {
    [self].into_iter().collect()
  }
}

impl ToCoapValue for u16 {
  fn to_coap_value<T: Array<Item = u8>>(self) -> T {
    self.to_be_bytes().into_iter().collect()
  }
}

impl ToCoapValue for u32 {
  fn to_coap_value<T: Array<Item = u8>>(self) -> T {
    self.to_be_bytes().into_iter().collect()
  }
}

impl ToCoapValue for u64 {
  fn to_coap_value<T: Array<Item = u8>>(self) -> T {
    self.to_be_bytes().into_iter().collect()
  }
}

macro_rules! builder_method {
  (
    #[doc = $doc:expr]
    #[option(num = $nr:literal)]
    fn $name:ident<$cfg:ty>(string);
  ) => {
    #[doc = $doc]
    pub fn $name<S: AsRef<str>>(self, value: S) -> Self {
      self.option(OptNumber($nr), value.as_ref())
    }
  };
  (
    #[doc = $doc:expr]
    #[option(num = $nr:literal)]
    fn $name:ident<$cfg:ty>(());
  ) => {
    #[doc = $doc]
    pub fn $name<S: AsRef<str>>(self) -> Self {
      self.option(OptNumber($nr), &*<$cfg>::MessageOptionBytes::default())
    }
  };
  (
    #[doc = $doc:expr]
    #[option(repeatable, num = $nr:literal)]
    fn $name:ident<$cfg:ty>(string);
  ) => {
    #[doc = $doc]
    pub fn $name<S: AsRef<str>>(self, value: S) -> Self {
      self.add_option(OptNumber($nr), value.as_ref())
    }
  };
  (
    #[doc = $doc:expr]
    #[option(num = $nr:literal)]
    fn $name:ident<$cfg:ty>($t:ty);
  ) => {
    #[doc = $doc]
    pub fn $name(self, value: $t) -> Self {
      self.option(OptNumber($nr), value)
    }
  };
  (
    #[doc = $doc:expr]
    #[option(repeatable, num = $nr:literal)]
    fn $name:ident<$cfg:ty>($t:ty);
  ) => {
    #[doc = $doc]
    pub fn $name(self, value: $t) -> Self {
      self.add_option(OptNumber($nr), value)
    }
  };
}

macro_rules! common_options {
  ($cfg:ty) => {
    crate::option::builder_method! {
      #[doc = toad_macros::rfc_7252_doc!("5.10.1")]
      #[option(num = 3)]
      fn host<$cfg>(string);
    }
    crate::option::builder_method! {
      #[doc = "see [`Self.host()`](#method.host)"]
      #[option(num = 11)]
      fn path<$cfg>(string);
    }
    crate::option::builder_method! {
      #[doc = "see [`Self.host()`](#method.host)"]
      #[option(num = 7)]
      fn port<$cfg>(u16);
    }
    crate::option::builder_method! {
      #[doc = "see [`Self.host()`](#method.host)"]
      #[option(repeatable, num = 15)]
      fn add_query<$cfg>(string);
    }
    crate::option::builder_method! {
      #[doc = toad_macros::rfc_7252_doc!("5.10.9")]
      #[option(num = 60)]
      fn size1<$cfg>(u32);
    }
    crate::option::builder_method! {
      #[doc = toad_macros::rfc_7252_doc!("5.10.8.1")]
      #[option(repeatable, num = 1)]
      fn if_match<$cfg>(tinyvec::ArrayVec<[u8; 8]>);
    }
    crate::option::builder_method! {
      #[doc = toad_macros::rfc_7252_doc!("5.10.8.2")]
      #[option(num = 5)]
      fn if_none_match<$cfg>(());
    }
    crate::option::builder_method! {
      #[doc = toad_macros::rfc_7252_doc!("5.10.2")]
      #[option(num = 35)]
      fn proxy_uri<$cfg>(string);
    }
    crate::option::builder_method! {
      #[doc = "See docs for [`Self.proxy_uri()`](#method.proxy_uri)"]
      #[option(num = 39)]
      fn proxy_scheme<$cfg>(string);
    }
    crate::option::builder_method! {
      #[doc = toad_macros::rfc_7252_doc!("5.10.5")]
      #[option(num = 14)]
      fn max_age<$cfg>(u32);
    }
    crate::option::builder_method! {
      #[doc = "See docs for [`Self.location_path()`](#method.location_path)"]
      #[option(repeatable, num = 20)]
      fn location_query<$cfg>(string);
    }
    crate::option::builder_method! {
      #[doc = toad_macros::rfc_7252_doc!("5.10.7")]
      #[option(repeatable, num = 8)]
      fn location_path<$cfg>(string);
    }
    crate::option::builder_method! {
      #[doc = concat!(
                toad_macros::rfc_7252_doc!("5.10.6"),
                "\n<details><summary>ETag as a Request Option</summary>\n\n",
                toad_macros::rfc_7252_doc!("5.10.6.2"),
                "\n</details><details><summary>ETag as a Response Option</summary>\n\n",
                toad_macros::rfc_7252_doc!("5.10.6.1"),
                "</details>"
      )]
      #[option(repeatable, num = 4)]
      fn etag<$cfg>(tinyvec::ArrayVec<[u8; 8]>);
    }
    crate::option::builder_method! {
      #[doc = toad_macros::rfc_7252_doc!("5.10.3")]
      #[option(num = 12)]
      fn content_format<$cfg>(crate::ContentFormat);
    }
    crate::option::builder_method! {
      #[doc = toad_macros::rfc_7252_doc!("5.10.4")]
      #[option(num = 17)]
      fn accept<$cfg>(crate::ContentFormat);
    }
  };
}

pub(crate) use {builder_method, common_options};
