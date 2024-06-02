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
use syn::parse::Parser as _;
use syn::punctuated::Punctuated;
use syn::Attribute;
use syn::Error;
use syn::Ident;
use syn::ItemFn;
use syn::Meta;
use syn::Result;
use syn::Token;


/// Our representation of a list of tags.
type Tags = Punctuated<Ident, Token![,]>;


/// Internally used marker for indicating the first macro expansion.
const FIRST_EXPANSION_MARKER: &str =
  "__internal_super_secret_sidekick_tag_do_not_use_or_you_may_regret";


/// A procedural macro for the `tag` attribute.
///
/// The attribute can be used to associate one or more tags with a test.
///
/// # Example
///
/// Specify the attribute on a per-test basis:
/// ```rust,no_run
/// #[test_tag::tag(miri)]
/// #[test]
/// fn compatible() {
///   assert_eq!(2 + 2, 4);
/// }
/// ```
#[proc_macro_attribute]
pub fn tag(attrs: TokenStream, item: TokenStream) -> TokenStream {
  try_tag(attrs, item)
    .unwrap_or_else(Error::into_compile_error)
    .into()
}


/// Handle the `#[test_tag::tag]` attribute.
///
/// The input to the function, for the following example:
/// ```rust,no_run
/// #[test_tag::tag(tag1, tag2)]
/// #[test_tag::tag(tag3)]
/// #[test]
/// fn it_works() {
///   assert_eq!(2 + 2, 4);
/// }
/// ```
/// would be:
/// - `attrs`: `tag1, tag2`
/// - `item`: `#[test_tag::tag(tag3)] #[test] fn it_works() { assert_eq!(2 + 2, 4); }`
fn try_tag(attrs: TokenStream, item: TokenStream) -> Result<Tokens> {
  // Parse the list of tags directly provided to *this* macro
  // instantiation.
  let mut tags = parse_tags(attrs)?;
  let input = ItemFn::parse.parse(item)?;
  let ItemFn {
    attrs,
    vis,
    mut sig,
    block,
  } = input;

  // Now also parse the attributes of the annotated function and filter
  // out any additional `test_tag::tag` candidates, parsing their tags
  // as well.
  let (more_tags, attrs) = parse_fn_attrs(attrs)?;
  let () = tags.extend(more_tags);

  let mut pairs = tags.into_iter();
  // SANITY: `parse_tags` makes sure that at least one tag is present.
  let tag = pairs.next().unwrap();
  let mut tags = Punctuated::from_iter(pairs);
  let first = is_first_expansion(&tags);
  if first {
    let () = tags.push(Ident::new(FIRST_EXPANSION_MARKER, Span::call_site()));
  }

  let last = tags.len() == 1;
  let tag_attrs = if last {
    debug_assert!(
      tags.iter().all(|tag| *tag == FIRST_EXPANSION_MARKER),
      "{}",
      quote! {#tags}
    );
    quote! {}
  } else {
    // If there are additional tags we just emit another `test_tag::tag`
    // attribute and let the expansion happen recursively. That may seem
    // complicated, but keep in mind that users are already able to use
    // multiple `test_tag::tag` attributes with one or more tags, so we
    // have to support this case anyway.
    quote! {
      #[::test_tag::tag(#tags)]
    }
  };

  // Save the name of the test before it is possibly being modified.
  let test_name = sig.ident.clone();
  if last {
    // Rename the test function to simply `test`. That's less confusing
    // than re-using the original name, which was already used in the
    // first module that we created.
    sig.ident = Ident::new("test", sig.ident.span());
  }

  let tagged = quote! {
    mod #tag {
      #tag_attrs
      #(#attrs)*
      #vis #sig {
        #block
      }
    }
  };

  let result = if first {
    // In the very first expansion we create a module using the test
    // name. In so doing we make sure that tags are always surrounded by
    // `::` in the final test name that the testing infrastructure
    // infers.
    quote! {
      mod #test_name {
        #tagged
      }
    }
  } else {
    tagged
  };
  Ok(result)
}

/// Parse a list of tags (`tag1, tag2`).
///
/// This function will report an error if the list is empty.
fn parse_tags(attrs: TokenStream) -> Result<Tags> {
  let tags = Tags::parse_terminated.parse(attrs)?;
  if !tags.is_empty() {
    Ok(tags)
  } else {
    Err(Error::new(
      Span::call_site(),
      "at least one tag is required: #[test_tag::tag(<tags...>)]",
    ))
  }
}

fn is_first_expansion(tags: &Tags) -> bool {
  // We always add the `FIRST_EXPANSION_MARKER` as the last tag, so it
  // will be sufficient to check that instead of scanning all of them.
  tags
    .iter()
    .next_back()
    .map(|tag| *tag != FIRST_EXPANSION_MARKER)
    .unwrap_or(true)
}

/// Parse the list of attributes to a function.
///
/// In the process, this function filters out anything resembling a
/// `tag` attribute and attempts to parsing its tags.
fn parse_fn_attrs(attrs: Vec<Attribute>) -> Result<(Tags, Vec<Attribute>)> {
  let mut tags = Tags::new();
  let mut passthrough_attrs = Vec::new();

  for attr in attrs {
    if is_test_tag_attr(&attr) {
      let tokens = match attr.meta {
        Meta::Path(..) => {
          // A path does not contain any tags. But leave error handling
          // up to the `parse_tags` function for consistency.
          quote! {}
        },
        Meta::List(list) => list.tokens,
        Meta::NameValue(..) => {
          return Err(Error::new_spanned(
            &attr,
            "encountered unexpected argument to `tag` attribute; expected list of tags",
          ))
        },
      };

      let attr_tags = parse_tags(tokens.into())?;
      tags = attr_tags.into_pairs().fold(tags, |mut tags, pair| {
        let () = tags.push(pair.into_value());
        tags
      });
    } else {
      let () = passthrough_attrs.push(attr);
    }
  }

  Ok((tags, passthrough_attrs))
}

/// Check whether given attribute is `#[tag]` or `#[test_tag::tag]`.
fn is_test_tag_attr(attr: &Attribute) -> bool {
  let path = match &attr.meta {
    // We conservatively treat an attribute without arguments as a
    // candidate as well, assuming it could just be wrong usage.
    Meta::Path(path) => path,
    Meta::List(list) => &list.path,
    _ => return false,
  };

  let segments = ["test_tag", "tag"];
  if path.leading_colon.is_none() && path.segments.len() == 1 && path.segments[0].ident == "tag" {
    true
  } else if path.segments.len() != segments.len() {
    false
  } else {
    path
      .segments
      .iter()
      .zip(segments)
      .all(|(segment, path)| segment.ident == path)
  }
}


#[cfg(test)]
mod tests {
  use super::*;


  /// Check that we can identify the `test_tag::tag` in different shapes
  /// and forms.
  #[test]
  fn test_tag_attr_recognition() {
    #[track_caller]
    fn test(func: Tokens) {
      let attrs = ItemFn::parse.parse2(func).unwrap().attrs;
      assert!(is_test_tag_attr(&attrs[0]));
      assert!(!is_test_tag_attr(&attrs[1]));
    }


    let func = quote! {
      #[tag(xxx)]
      #[test]
      fn foobar() {}
    };
    let () = test(func);

    let func = quote! {
      #[test_tag::tag(xxx)]
      #[test]
      fn foobar() {}
    };
    let () = test(func);

    let func = quote! {
      #[::test_tag::tag(xxx)]
      #[test]
      fn foobar() {}
    };
    let () = test(func);
  }
}
