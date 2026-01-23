// Copyright (C) 2024-2026 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: (Apache-2.0 OR MIT)

//! Tagging functionality for tests, allowing for convenient grouping and later
//! execution of certain groups.
//!
//! For example, a test can be associated with the tag `miri`, to
//! indicate that it is suitable for being run under
//! [Miri](https://github.com/rust-lang/miri):
//! ```rust,ignore
//! use test_tag::tag;
//!
//! #[tag(miri)]
//! #[test]
//! fn test1() {
//!   assert_eq!(2 + 2, 4);
//! }
//! ```
//!
//! Subsequently, it is possible to run only those tests under Miri:
//! ```sh
//! $ cargo miri test -- :miri:
//! ```
//!
//! Please note that the usage of Miri is just an example (if the
//! majority of tests is Miri-compatible you can use `#[cfg_attr(miri,
//! ignore)]` instead and may not require a custom attribute). However,
//! tagging can be useful for other properties, such as certain tests
//! requiring alleviated rights (need to be run with administrator
//! privileges).
//!
//! This crate provides the #[test_tag::[macro@tag]] attribute that allows for
//! such tagging to happen.

#![warn(missing_docs)]

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as Tokens;

use quote::quote;
use quote::quote_spanned;

use syn::parse::Parse;
use syn::parse::Parser as _;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned as _;
use syn::Attribute;
use syn::Error;
use syn::Ident;
use syn::ItemFn;
use syn::Meta;
use syn::MetaNameValue;
use syn::PathArguments;
use syn::PathSegment;
use syn::Result;
use syn::Token;


/// Our representation of a list of tags.
type Tags = Punctuated<Ident, Token![,]>;


/// A procedural macro for the `tag` attribute.
///
/// The attribute can be used to associate one or more tags with a test. The
/// attribute should be placed before the eventual `#[test]` attribute.
///
/// # Example
///
/// Specify the attribute on a per-test basis:
/// ```rust,ignore
/// use test_tag::tag;
///
/// #[tag(tag1, tag2)]
/// #[test]
/// fn test1() {
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
/// ```rust,ignore
/// use test_tag::tag;
///
/// #[tag(tag1, tag2)]
/// #[tag(tag3)]
/// #[test]
/// fn it_works() {
///   assert_eq!(2 + 2, 4);
/// }
/// ```
/// would be:
/// - `attrs`: `tag1, tag2`
/// - `item`: `#[tag(tag3)] #[test] fn it_works() { assert_eq!(2 + 2, 4); }`
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
  // in the process.
  let (more_tags, mut attrs) = parse_fn_attrs(attrs)?;
  let () = tags.extend(more_tags);
  let () = rewrite_test_attrs(&mut attrs);

  let test_name = sig.ident.clone();
  // Rename the test function to simply `test`. That's less confusing
  // than re-using the original name, which we intend to use in the
  // first module that we create.
  sig.ident = Ident::new("test", sig.ident.span());

  let mut result = quote! {
    #(#attrs)*
    pub #sig {
      #block
    }
  };

  let mut import = None;
  for tag in tags.into_iter().rev() {
    import = if let Some(import) = &import {
      Some(quote! { #tag::#import })
    } else {
      Some(quote! { #tag })
    };

    result = quote! {
      pub mod #tag {
        use super::*;
        #result
      }
    };
  }

  // Wrap everything in a module named after the test. In so doing we
  // make sure that tags are always surrounded by `::` in the final test
  // name that the testing infrastructure infers.
  // NB: We need to import the standard prelude here so that some
  //     `#[test]` attribute is present. That is necessary because we
  //     rewrite #[test] attributes on tagged functions to
  //     `#[self::test]` and then rely on *a* `#[test]` attribute being
  //     in scope. We cannot, however, import `core::prelude::v1::test`
  //     directly, because that would conflict with potential user
  //     imports.
  // TODO: We need to find an alternative solution to allowing the
  //       `ambiguous_panic_imports` lint here.
  result = quote! {
    use ::core::prelude::v1::*;
    #[allow(unused_imports)]
    #vis use #test_name::#import::test as #test_name;
    #[doc(hidden)]
    #[allow(ambiguous_panic_imports)]
    pub mod #test_name {
      use super::*;
      #result
    }
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
    Err(Error::new_spanned(
      &tags,
      "at least one tag is required: #[test_tag::tag(<tags...>)]",
    ))
  }
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
          quote_spanned!(attr.meta.span() => {})
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
      let () = tags.extend(attr_tags);
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


/// Rewrite remaining `#[test]` attributes to use `#[self::test]` syntax.
///
/// This conversion is necessary in order to properly support custom `#[test]`
/// attributes. These attributes are somewhat special and require custom
/// treatment, because Rust's prelude also contains such an attribute
/// and we run risk of ambiguities without this rewrite.
fn rewrite_test_attrs(attrs: &mut [Attribute]) {
  for attr in attrs.iter_mut() {
    let span = attr.meta.span();
    let path = match &mut attr.meta {
      Meta::Path(path) => path,
      Meta::List(list) => &mut list.path,
      Meta::NameValue(MetaNameValue { path, .. }) => path,
    };

    if path.leading_colon.is_none() && path.segments.len() == 1 && path.segments[0].ident == "test"
    {
      let segment = PathSegment {
        ident: Ident::new("self", span),
        arguments: PathArguments::None,
      };
      let () = path.segments.insert(0, segment);
    }
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

    let func = quote! {
      #[::test_tag::tag]
      #[test]
      fn foobar() {}
    };
    let () = test(func);

    let func = quote! {
      #[test_tag::tag]
      #[test]
      fn foobar() {}
    };
    let () = test(func);
  }
}
