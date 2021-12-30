//! # kwap_reqresp
//! High-level representation of CoAP requests and responses.

#![doc(html_root_url = "https://docs.rs/kwap-reqresp/0.1.0")]
#![cfg_attr(all(not(test), feature = "no_std"), no_std)]
#![cfg_attr(not(test), forbid(missing_debug_implementations, unreachable_pub))]
#![cfg_attr(not(test), deny(unsafe_code, missing_copy_implementations))]
#![cfg_attr(any(docsrs, feature = "docs"), feature(doc_cfg))]
#![deny(missing_docs)]

#[cfg(feature = "alloc")]
extern crate alloc as std_alloc;

use kwap_macros::rfc_7252_doc;

///
pub struct Req {}

///
pub struct Rep {}

macro_rules! code {
  ($section:literal $c:literal . $d:literal $name:ident) => {
    #[doc = rfc_7252_doc!($section)]
    pub const $name: kwap_msg::Code = kwap_msg::Code {class: $c, detail: $d};
  };
}

// 2.xx
code!("5.9.1.1"  2 . 01  CREATED);
code!("5.9.1.2"  2 . 02  DELETED);
code!("5.9.1.3"  2 . 03  VALID);
code!("5.9.1.4"  2 . 04  CHANGED);
code!("5.9.1.5"  2 . 05  CONTENT);

// 4.xx
code!("5.9.2.1"  4 . 00  BAD_REQUEST);
code!("5.9.2.2"  4 . 01  UNAUTHORIZED);
code!("5.9.2.3"  4 . 02  BAD_OPTION);
code!("5.9.2.4"  4 . 03  FORBIDDEN);
code!("5.9.2.5"  4 . 04  NOT_FOUND);
code!("5.9.2.6"  4 . 05  METHOD_NOT_ALLOWED);
code!("5.9.2.7"  4 . 06  NOT_ACCEPTABLE);
code!("5.9.2.8"  4 . 12  PRECONDITION_FAILED);
code!("5.9.2.9"  4 . 13  REQUEST_ENTITY_TOO_LARGE);
code!("5.9.2.10" 4 . 15  UNSUPPORTED_CONTENT_FORMAT);

// 5.xx
code!("5.9.3.1"  5 . 00  INTERNAL_SERVER_ERROR);
code!("5.9.3.2"  5 . 01  NOT_IMPLEMENTED);
code!("5.9.3.3"  5 . 02  BAD_GATEWAY);
code!("5.9.3.4"  5 . 03  SERVICE_UNAVAILABLE);
code!("5.9.3.5"  5 . 04  GATEWAY_TIMEOUT);
code!("5.9.3.6"  5 . 05  PROXYING_NOT_SUPPORTED);
