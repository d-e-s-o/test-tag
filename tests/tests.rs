// Copyright (C) 2024 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: (Apache-2.0 OR MIT)

mod common;

use common::run_tests;

use maplit::hashset;


fn some_impl() {}


#[test_tag::tag(tag1)]
#[test]
#[ignore]
fn test1() {}


#[test_tag::tag(tag1, tag2)]
#[test]
#[ignore]
fn test2() {
  // Make sure that we can reference another test.
  let () = test1();
}


#[rustfmt::skip]
#[test_tag::tag(tag1)]
#[test_tag::tag(tag3,)]
#[test]
#[ignore]
fn test3() {
  // Make sure that we can reference an ordinarily accessible function.
  let () = some_impl();
}


#[test_tag::tag(ignore_tag)]
#[test]
#[ignore]
fn test4() {
  panic!()
}


// Check that macro expansion work with a custom `#[test]` attribute.
mod namespace1 {
  use test_log::test;


  #[test_tag::tag(tag3)]
  #[test]
  #[ignore]
  fn test5() {}
}


// Check that macro expansion work with a custom `#[test]` attribute for
// async functions.
mod namespace2 {
  use tokio::test;


  #[test_tag::tag(tag3)]
  #[test]
  #[ignore]
  async fn test6() {}


  #[test_tag::tag(tag3)]
  #[tokio::test]
  #[ignore]
  async fn test7() {}
}


// Check that we can use abbreviated `tag` import as well.
mod namespace3 {
  use test_tag::tag;


  #[tag(tag3)]
  #[test]
  #[ignore]
  fn test8() {}
}


/// The only default-runnable test. It recursively invokes the binary to
/// check that ignored tests have the expected set of tags.
#[test]
fn main() {
  {
    let tests = run_tests(&[":tag1:"]).unwrap();
    let expected = hashset! {
      "test1::tag1::test".to_string(),
      "test2::tag1::tag2::test".to_string(),
      "test3::tag1::tag3::test".to_string(),
    };
    assert_eq!(tests, expected);
  }

  {
    let tests = run_tests(&[":tag2:"]).unwrap();
    let expected = hashset! {
      "test2::tag1::tag2::test".to_string(),
    };
    assert_eq!(tests, expected);
  }

  {
    let tests = run_tests(&[":tag3:"]).unwrap();
    let expected = hashset! {
      "test3::tag1::tag3::test".to_string(),
      "namespace1::test5::tag3::test".to_string(),
      "namespace2::test6::tag3::test".to_string(),
      "namespace2::test7::tag3::test".to_string(),
      "namespace3::test8::tag3::test".to_string(),
    };
    assert_eq!(tests, expected);
  }
}
