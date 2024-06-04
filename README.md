[![pipeline](https://github.com/d-e-s-o/test-tag/actions/workflows/test.yml/badge.svg?branch=main)](https://github.com/d-e-s-o/test-tag/actions/workflows/test.yml)
[![crates.io](https://img.shields.io/crates/v/test-tag.svg)](https://crates.io/crates/test-tag)
[![Docs](https://docs.rs/test-tag/badge.svg)](https://docs.rs/test-tag)

test-tag
========

- [Documentation][docs-rs]

**test-tag** is a crate that that can be used for tagging tests. Users
are then able to execute only tests matching certain tags.

Problem
-------
Rust makes it very easy to define tests at all layers of the
application/library. But not all tests are created equal and sometimes
it is necessary to highlight certain properties and have the
corresponding tests be treated differently.

A common example is testing with [Miri][miri]: it can run certain tests,
but the moment a test performs file IO or crosses FFI boundaries it
becomes ineligible to be run under Miri. As such, just running `cargo
miri test` on any non-trivial crate is unlikely to work, as at least
some tests are likely to violate these constraints.

Workarounds include, for example, including `miri` in the test name and
then filtering tests at the invocation level; say, `cargo miri test --
_miri_`. But that is not a particularly obvious convention and so it
is entirely possible that a contributor accidentally renames a test
rendering it no longer eligible to be run. It also quickly gets
convoluted once more than one property is "special".

Usage
-----
This crate provides the `#[test_tag::tag(...)]` attribute that
introduces the means for first class tagging. For the `Miri` example:
```rust
#[test_tag::tag(miri)]
#[test]
fn test1() {}
```

One would then be able to run it via:
```sh
$ cargo miri test -- :miri:
```

Tests can also be excluded based on tags. Let's say some tests are
taking a long time and you would not want to run them under `Miri` *and*
natively. You can exclude all `Miri` tests easily via:
```sh
$ cargo test -- --skip :miri:
```

#### Multiple Tags
One can provide a list of tags, either in comma separated form or by
providing the attribute multiple times:
```rust
#[test_tag::tag(tag1, tag2)]
#[test]
fn test1() {}

// The above is equivalent to:

#[test_tag::tag(tag1)]
#[test_tag::tag(tag2)]
#[test]
fn test1() {}
```

#### Limitations
Note, however, that limitations of Rust's test framework may mean that
you may not be able to express arbitrary constraints on tags. For
example, a standard unit test won't let you specify a conjunction of two
tags:
```sh
$ cargo test -- :tag1: :tag2:
```
The above will be interpreted as "run all tests that have `:tag1:` *or*
`:tag2:` (or both)".

[docs-rs]: https://docs.rs/test-log/latest/test_tag/
[miri]: https://github.com/rust-lang/miri
