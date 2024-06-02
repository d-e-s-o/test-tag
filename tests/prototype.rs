// Copyright (C) 2024 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: (Apache-2.0 OR MIT)

//! Use this file for iterating on the derive code. You can view the
//! expanded code for any given configuration by updating this file and
//! running:
//!
//! ```sh
//! cargo expand --test=prototype
//! ```


#[test_tag::tag(tag1, tag2)]
#[test_tag::tag(tag3)]
#[test]
fn it_works() {
  assert_eq!(2 + 2, 4);
}

#[test_tag::tag(tag2)]
#[test]
fn it_works_as_well() {
  assert_eq!(2 + 2, 4);
}
