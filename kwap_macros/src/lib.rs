//! Macros used by `kwap` for boilerplate reduction

#![doc(html_root_url = "https://docs.rs/kwap-macros/0.1.5")]
#![cfg_attr(all(not(test), feature = "no_std"), no_std)]
#![cfg_attr(not(test), forbid(missing_debug_implementations, unreachable_pub))]
#![cfg_attr(not(test), deny(unsafe_code, missing_copy_implementations))]
#![cfg_attr(any(docsrs, feature = "docs"), feature(doc_cfg))]
#![deny(missing_docs)]

use proc_macro::TokenStream;
use quote::ToTokens;
use regex::Regex;
use syn::{parse::Parse, parse_macro_input, LitStr};

struct DocSection(LitStr);

impl Parse for DocSection {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    Ok(Self(input.parse::<LitStr>()?))
  }
}

const RFC7252: &str = include_str!("./rfc7252.txt");

/// Give me a section of RFC7252 (e.g. `5.9.1.1` no trailing dot)
/// and I will scrape the rfc for that section then yield an inline `#[doc]` attribute containing that section.
///
/// ```
/// use kwap_macros::rfc_7252_doc;
///
/// #[doc = rfc_7252_doc!("5.9.1.1")]
/// // Expands to:
/// /// # 2.04 Changed
/// /// [_generated from RFC7252 section 5.9.1.1_](<link to section at ietf.org>)
/// ///
/// /// This Response Code is like HTTP 204 "No Content" but only used in
/// /// response to POST and PUT requests.  The payload returned with the
/// /// response, if any, is a representation of the action result.
/// ///
/// /// This response is not cacheable.  However, a cache MUST mark any
/// /// stored response for the changed resource as not fresh.
/// struct Foo;
/// ```
#[proc_macro]
pub fn rfc_7252_doc(input: TokenStream) -> TokenStream {
  let DocSection(section_literal) = parse_macro_input!(input as DocSection);

  let sec = section_literal.value();
  let docstring = gen_docstring(sec, RFC7252);

  LitStr::new(&docstring, section_literal.span()).to_token_stream().into()
}

fn gen_docstring(sec: String, rfc: &'static str) -> String {
  // Match {beginning of line}{section number} then capture everything until beginning of next section
  let section_rx =
    Regex::new(format!(r"(?s)\n{}\.\s+(.*?)(\n\d|$)", sec.replace(".", "\\.")).as_str()).unwrap_or_else(|e| {
                                                                                      panic!("Section {} invalid: {:?}", sec, e)
                                                                                    });
  let rfc_section = section_rx.captures_iter(rfc)
                              .next()
                              .unwrap_or_else(|| panic!("Section {} not found", sec))
                              .get(1)
                              .unwrap_or_else(|| panic!("Section {} is empty", sec))
                              .as_str();

  let mut lines = trim_leading_ws(rfc_section);
  let line1 = lines.drain(0..1)
                   .next()
                   .unwrap_or_else(|| panic!("Section {} is empty", sec));
  let rest = lines.join("\n");

  format!(
          r"# {title}
[_generated from RFC7252 section {section}_](https://datatracker.ietf.org/doc/html/rfc7252#section-{section})

{body}",
          title = line1,
          section = sec,
          body = rest
  )
}

/// the RFC is formatted with 3-space indents in section bodies, with some addl
/// indentation on some text.
///
/// This strips all leading whitespaces, except within code fences (&#96;&#96;&#96;), where it just trims the 3-space indent.
///
/// Returns the input string split by newlines
fn trim_leading_ws(text: &str) -> Vec<String> {
  #[derive(Clone, Copy)]
  enum TrimStart {
    Yes,
    InCodeFence,
  }

  let trim_start = Regex::new(r"^ +").unwrap();
  let trim_indent = Regex::new(r"^   ").unwrap();

  text.split('\n')
      .fold((Vec::<String>::new(), TrimStart::Yes), |(mut lines, strip), s| {
        let trimmed = trim_start.replace(s, "").to_string();
        let dedented = trim_indent.replace(s, "").to_string();

        let is_fence = trimmed.starts_with("```");

        match (is_fence, strip) {
          | (false, TrimStart::Yes) => {
            lines.push(trimmed);
            (lines, strip)
          },
          | (false, TrimStart::InCodeFence) => {
            lines.push(dedented);
            (lines, strip)
          },
          | (true, TrimStart::Yes) => {
            lines.push(trimmed);
            (lines, TrimStart::InCodeFence)
          },
          | (true, TrimStart::InCodeFence) => {
            lines.push(trimmed);
            (lines, TrimStart::Yes)
          },
        }
      })
      .0
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rfcdoc_works() {
    let rfc = r"
Table of Contents

   1.  Foo . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . 1
     1.1.  Bingus .  . . . . . . . . . . . . . . . . . . . . . . . . . 2
     1.2.  Terminology . . . . . . . . . . . . . . . . . . . . . . . . 3
   2.  Bar . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . 4

1. Foo
   bar baz quux

   ```text
   dingus bar
     foo
   ```
1.1.    Bingus
   lorem ipsum frisky gypsum

1.2. Terminology
   Bat: tool used for baseball
   Code: if (name === 'Jerry') {throw new Error('get out jerry!!1');}

2. Bar
   bingus
   o fart
   o poo";
    // preserves whitespace, finds end of section that is not last
    assert_eq!(
               gen_docstring("1".into(), rfc),
               r"# Foo
[_generated from RFC7252 section 1_](https://datatracker.ietf.org/doc/html/rfc7252#section-1)

bar baz quux

```text
dingus bar
  foo
```"
    );

    // finds end of section that is last
    assert_eq!(
               gen_docstring("2".into(), rfc),
               r"# Bar
[_generated from RFC7252 section 2_](https://datatracker.ietf.org/doc/html/rfc7252#section-2)

bingus
o fart
o poo"
    );
  }
}
