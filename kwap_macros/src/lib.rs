//! # kwap_macros
//! Macros used by `kwap` for boilerplate reduction

#![doc(html_root_url = "https://docs.rs/kwap-macros/0.1.0")]
#![cfg_attr(all(not(test), feature = "no_std"), no_std)]
#![cfg_attr(not(test), forbid(missing_debug_implementations, unreachable_pub))]
#![cfg_attr(not(test), deny(unsafe_code, missing_copy_implementations))]
#![cfg_attr(any(docsrs, feature = "docs"), feature(doc_cfg))]
#![deny(missing_docs)]

use proc_macro::TokenStream;
use quote::ToTokens;
use regex::Regex;
use syn::{parse_macro_input, parse::Parse, LitStr};

struct DocSection(LitStr);

impl Parse for DocSection {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
      Ok(Self(input.parse::<LitStr>()?))
  }
}

const RFC7252: &'static str = include_str!("./rfc7252.txt");

/// Give me a section of RFC7252 (e.g. `5.9.1.1` no trailing dot)
/// and I will scrape the rfc for that section then yield an inline `#[doc]` attribute containing that section.
#[proc_macro]
pub fn rfc_7252_doc(input: TokenStream) -> TokenStream {
  let DocSection(section_literal) = parse_macro_input!(input as DocSection);

  let sec = section_literal.value();

  // Match {beginning of line}{section number} then capture everything until beginning of next section
  let section_rx = Regex::new(format!(r"(?sm)^{}\.\s+(.*?)\n\d", sec.replace(".", "\\.")).as_str()).expect(&format!("Section {} invalid", sec));
  let rfc_section = section_rx.captures_iter(RFC7252).next().expect(&format!("Section {} not found", sec)).get(1).expect(&format!("Section {} is empty", sec)).as_str();

  // remove leading spaces + separate first line (title of section) from the rest (section body)
  let mut lines = rfc_section.split('\n').map(|s| Regex::new(r"^ +").unwrap().replace(s, ""));
  let line1 = lines.next().unwrap();
  let rest = lines.collect::<Vec<_>>().join("\n");

  let docstring = format!(
      r"# {title}
[_generated from RFC7252 section {section}_](https://datatracker.ietf.org/doc/html/rfc7252#section-{section})

{body}", title = line1, section = sec, body = rest);
  LitStr::new(&docstring, section_literal.span()).to_token_stream().into()
}
