#[cfg(feature = "alloc")]
use std_alloc::string::{String, ToString};
use toad_macros::rfc_7252_doc;

#[doc = rfc_7252_doc!("12.1")]
/// <details><summary><b>RFC7252 Section 12.1.1 Method Codes</b></summary>
#[doc = concat!("\n#", rfc_7252_doc!("12.1.1"))]
/// </details>
/// <details><summary><b>RFC7252 Section 12.1.2 Response Codes</b></summary>
#[doc = concat!("\n#", rfc_7252_doc!("12.1.2"))]
/// </details>
///
/// # Examples
/// ```
/// use toad_msg::Code;
///
/// assert_eq!(Code { class: 2,
///                   detail: 5 }.to_string(),
///            "2.05".to_string());
/// ```
#[derive(Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Code {
  /// The "class" of message codes identify it as a request or response, and provides the class of response status:
  ///
  /// |class|meaning|
  /// |---|---|
  /// |`0`|Message is a request|
  /// |`2`|Message is a success response|
  /// |`4`|Message is a client error response|
  /// |`5`|Message is a server error response|
  pub class: u8,

  /// 2-digit integer (range `[0, 32)`) that provides granular information about the response status.
  ///
  /// Will always be `0` for requests.
  pub detail: u8,
}

/// Whether a code is for a request, response, or empty message
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodeKind {
  /// A request code (0.xx)
  Request,
  /// A response code ([2-5].xx)
  Response,
  /// EMPTY (0.00)
  Empty,
}

impl Code {
  /// Create a new Code
  ///
  /// ```
  /// use toad_msg::Code;
  ///
  /// let content = Code::new(2, 05);
  /// ```
  pub const fn new(class: u8, detail: u8) -> Self {
    Self { class, detail }
  }

  /// Get the human string representation of a message code
  ///
  /// # Returns
  /// A `char` array
  ///
  /// This is to avoid unnecessary heap allocation,
  /// you can create a `String` with `FromIterator::<String>::from_iter`,
  /// or if the `alloc` feature of `toad` is enabled there is a `ToString` implementation provided for Code.
  /// ```
  /// use toad_msg::Code;
  ///
  /// let code = Code { class: 2,
  ///                   detail: 5 };
  /// let chars = code.to_human();
  /// let string = String::from_iter(chars);
  /// assert_eq!(string, "2.05".to_string());
  /// ```
  pub fn to_human(&self) -> [char; 4] {
    let to_char = |d: u8| char::from_digit(d.into(), 10).unwrap();
    [to_char(self.class),
     '.',
     to_char(self.detail / 10),
     to_char(self.detail % 10)]
  }

  /// Get whether this code is for a request, response, or empty message
  ///
  /// ```
  /// use toad_msg::{Code, CodeKind};
  ///
  /// let empty: Code = Code::new(0, 0);
  /// assert_eq!(empty.kind(), CodeKind::Empty);
  ///
  /// let req = Code::new(0, 1); // GET
  /// assert_eq!(req.kind(), CodeKind::Request);
  ///
  /// let resp = Code::new(2, 5); // OK CONTENT
  /// assert_eq!(resp.kind(), CodeKind::Response);
  /// ```
  pub fn kind(&self) -> CodeKind {
    match (self.class, self.detail) {
      | (0, 0) => CodeKind::Empty,
      | (0, _) => CodeKind::Request,
      | _ => CodeKind::Response,
    }
  }

  #[doc = rfc_7252_doc!("4.1")]
  pub const EMPTY: Self = Self::new(0, 0);

  #[doc = rfc_7252_doc!("5.8.1")]
  pub const GET: Self = Self::new(0, 1);

  #[doc = rfc_7252_doc!("5.8.2")]
  pub const POST: Self = Self::new(0, 3);

  #[doc = rfc_7252_doc!("5.8.3")]
  pub const PUT: Self = Self::new(0, 2);

  #[doc = rfc_7252_doc!("5.8.4")]
  pub const DELETE: Self = Self::new(0, 4);
}

#[cfg(feature = "alloc")]
#[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
impl ToString for Code {
  fn to_string(&self) -> String {
    String::from_iter(self.to_human())
  }
}

impl From<u8> for Code {
  fn from(b: u8) -> Self {
    // xxxyyyyy

    // xxx => class
    let class = b >> 5;

    // yyyyy => detail
    let detail = b & 0b00011111;

    Code { class, detail }
  }
}

impl From<Code> for u8 {
  fn from(code: Code) -> u8 {
    let class = (code.class << 5) & 0b11100000;
    let detail = code.detail & 0b00011111;

    class | detail
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::assert_eqb;

  #[test]
  fn parse_code() {
    let byte = 0b01000101_u8;
    let code = Code::from(byte);
    assert_eq!(code,
               Code { class: 2,
                      detail: 5 })
  }

  #[test]
  fn serialize_code() {
    let code = Code { class: 2,
                      detail: 5 };
    let actual: u8 = code.into();
    let expected = 0b01000101_u8;
    assert_eqb!(actual, expected)
  }
}
