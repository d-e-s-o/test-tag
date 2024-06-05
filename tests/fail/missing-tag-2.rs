// Copyright (C) 2024 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: (Apache-2.0 OR MIT)

/// Missing tags in second attribute.
#[test_tag::tag(tag)]
#[test_tag::tag]
#[test]
fn cant_compile() {}
