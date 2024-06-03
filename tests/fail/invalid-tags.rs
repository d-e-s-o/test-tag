// Copyright (C) 2024 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: (Apache-2.0 OR MIT)

/// Invalid specifiers in tag list.
#[test_tag::tag(abc!@*()!@)]
#[test]
fn cant_compile() {}
