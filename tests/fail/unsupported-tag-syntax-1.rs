// Copyright (C) 2024 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: (Apache-2.0 OR MIT)

/// Unsupported key-value syntax.
#[test_tag::tag(tag1 = "foobar")]
#[test]
fn cant_compile() {}
