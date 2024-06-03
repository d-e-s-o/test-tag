// Copyright (C) 2024 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use trybuild::TestCases;


/// Make sure that certain wrong attribute usages are caught at compile
/// time.
#[test]
fn failures() {
  let t = TestCases::new();
  let () = t.compile_fail("tests/fail/*.rs");
}
