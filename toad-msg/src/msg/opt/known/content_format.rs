/// Content-Format
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

impl From<u16> for ContentFormat {
  fn from(n: u16) -> Self {
    use ContentFormat::*;
    match n {
      | 0 => Text,
      | 40 => LinkFormat,
      | 41 => Xml,
      | 42 => OctetStream,
      | 47 => Exi,
      | 50 => Json,
      | n => Other(n),
    }
  }
}

impl<'a> IntoIterator for &'a ContentFormat {
  type Item = u8;

  type IntoIter = <[u8; 2] as IntoIterator>::IntoIter;

  fn into_iter(self) -> Self::IntoIter {
    self.bytes().into_iter()
  }
}
