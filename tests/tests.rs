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
fn test2() {}


#[rustfmt::skip]
#[test_tag::tag(tag1)]
#[test_tag::tag(tag3,)]
#[test]
#[ignore]
fn test3() {
    some_impl()
}


#[test_tag::tag(ignore_tag)]
#[test]
#[ignore]
fn test4() {
  panic!()
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
    };
    assert_eq!(tests, expected);
  }
}
