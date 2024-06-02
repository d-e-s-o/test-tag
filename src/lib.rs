// Copyright (C) 2024 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: (Apache-2.0 OR MIT)

//! Tagging functionality for tests, allowing for convenient grouping.
//!
//! For example, a test can be associated with the tag `miri`, to
//! indicate that it is suitable for being run under
//! [Miri](https://github.com/rust-lang/miri):
//!
//! A crate providing a replacement #[[macro@tag]] attribute that
//! initializes logging and/or tracing infrastructure before running
//! tests.

#![warn(missing_docs)]

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro2::TokenStream as Tokens;

use quote::quote;

use syn::parse::Parse;
use syn::parse_macro_input;
use syn::Attribute;
use syn::Error;
use syn::ItemFn;
use syn::Meta;
use syn::Result;


/// A procedural macro for the `tag` attribute.
///
/// The attribute can be used to associate one or more tags with a test.
///
/// # Example
///
/// Specify the attribute on a per-test basis:
/// ```rust
/// #[test_tag::tag(miri)]
/// fn compatible() {
///   assert_eq!(2 + 2, 4);
/// }
/// ```
#[proc_macro_attribute]
pub fn tag(attr: TokenStream, item: TokenStream) -> TokenStream {
  let item = parse_macro_input!(item as ItemFn);
  try_tag(attr, item)
    .unwrap_or_else(Error::into_compile_error)
    .into()
}


fn try_tag(attr: TokenStream, input: ItemFn) -> Result<Tokens> {
  println!("ATTR: {}", attr);
  let ItemFn {
    attrs,
    vis,
    sig,
    block,
  } = input;

  let (tag, ignored_attrs) = parse_tag(attr)?;
  let (inner_test, generated_test) = if attr.is_empty() {
    let has_test = ignored_attrs.iter().any(is_test_attribute);
    let generated_test = if has_test {
      quote! {}
    } else {
      quote! { #[::core::prelude::v1::test]}
    };
    (quote! {}, generated_test)
  } else {
    let attr = Tokens::from(attr);
    (quote! { #[#attr] }, quote! {})
  };

  let result = quote! {
    mod #tag {
      #inner_test
      #(#ignored_attrs)*
      #generated_test
      #vis #sig {
        #block
      }
    }
  };
  Ok(result)
}


// Check whether given attribute is `#[test]` or `#[::core::prelude::v1::test]`.
fn is_test_attribute(attr: &Attribute) -> bool {
  let path = match &attr.meta {
    Meta::Path(path) => path,
    _ => return false,
  };
  let segments = ["core", "prelude", "v1", "test"];
  if path.leading_colon.is_none() {
    return path.segments.len() == 1
      && path.segments[0].arguments.is_none()
      && path.segments[0].ident == "test";
  } else if path.segments.len() != segments.len() {
    return false;
  }
  path
    .segments
    .iter()
    .zip(segments)
    .all(|(segment, path)| segment.arguments.is_none() && segment.ident == path)
}


fn parse_tag(attrs: Vec<Attribute>) -> Result<(String, Vec<Attribute>)> {
  let mut tag = None;
  let mut ignored_attrs = Vec::new();
  for attr in attrs {
    if let Some(parsed_tag) = try_parse_tag(&attr)? {
      if tag.replace(parsed_tag).is_some() {
        return Err(Error::new_spanned(
          &attr,
          "encountered more than one tag (`{parsed_tag}`)",
        ))
      }
    } else {
      // Anything we failed to parse as a tag is just kept verbatim for
      // inclusion in the generated code.
      let () = ignored_attrs.push(attr);
    }
  }

  let tag = tag.ok_or_else(|| {
    // XXX: Check whether the span is remotely correct.
    Error::new(
      Span::call_site(),
      "a tag is missing for #[test_tag::tag] attribute",
    )
  })?;

  Ok((tag, ignored_attrs))
}


fn try_parse_tag(attr: &Attribute) -> Result<Option<String>> {
  if !attr.path().is_ident("test_tag") {
    return Ok(None)
  }

  let nested_meta = attr.parse_args_with(Meta::parse)?;
  let list = if let Meta::List(list) = nested_meta {
    list.tokens
  } else {
    return Err(Error::new_spanned(
      &nested_meta,
      "Expected NameValue syntax, e.g. 'default_log_filter = \"debug\"'.",
    ))
  };
  Ok(Some(list.to_string()))
}
