// Copyright (C) 2024 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use std::collections::HashSet;
use std::env::current_exe;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::io::BufRead as _;
use std::process::Command;
use std::process::Output;
use std::process::Stdio;

use anyhow::bail;
use anyhow::Context as _;
use anyhow::Result;

use serde::Deserialize;
use serde_json::from_str as from_json;


/// Concatenate a command and its arguments into a single string.
fn concat_command<C, A, S>(command: C, args: A) -> OsString
where
  C: AsRef<OsStr>,
  A: IntoIterator<Item = S>,
  S: AsRef<OsStr>,
{
  args
    .into_iter()
    .fold(command.as_ref().to_os_string(), |mut cmd, arg| {
      cmd.push(OsStr::new(" "));
      cmd.push(arg.as_ref());
      cmd
    })
}


/// Format a command with the given list of arguments as a string.
fn format_command<C, A, S>(command: C, args: A) -> String
where
  C: AsRef<OsStr>,
  A: IntoIterator<Item = S>,
  S: AsRef<OsStr>,
{
  concat_command(command, args).to_string_lossy().to_string()
}


fn evaluate<C, A, S>(output: &Output, command: C, args: A) -> Result<()>
where
  C: AsRef<OsStr>,
  A: IntoIterator<Item = S>,
  S: AsRef<OsStr>,
{
  if !output.status.success() {
    let code = if let Some(code) = output.status.code() {
      format!(" ({code})")
    } else {
      " (terminated by signal)".to_string()
    };

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stderr = stderr.trim_end();
    let stderr = if !stderr.is_empty() {
      format!(": {stderr}")
    } else {
      String::new()
    };

    bail!(
      "`{}` reported non-zero exit-status{code}{stderr}",
      format_command(command, args),
    );
  }
  Ok(())
}

/// Run a command with the provided arguments.
fn run_impl<C, A, S>(command: C, args: A, stdout: Stdio) -> Result<Output>
where
  C: AsRef<OsStr>,
  A: IntoIterator<Item = S> + Clone,
  S: AsRef<OsStr>,
{
  let output = Command::new(command.as_ref())
    .stdin(Stdio::null())
    .stdout(stdout)
    .args(args.clone())
    .output()
    .with_context(|| {
      format!(
        "failed to run `{}`",
        format_command(command.as_ref(), args.clone())
      )
    })?;

  let () = evaluate(&output, command, args)?;
  Ok(output)
}

/// Run a command and capture its output.
fn output<C, A, S>(command: C, args: A) -> Result<Vec<u8>>
where
  C: AsRef<OsStr>,
  A: IntoIterator<Item = S> + Clone,
  S: AsRef<OsStr>,
{
  let output = run_impl(command, args, Stdio::piped())?;
  Ok(output.stdout)
}


#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Event {
  Started,
  Ok,
  Ignored,
  Failed,
}


#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum TestLine {
  Suite,
  Test { name: String, event: Event },
}


/// Run all tests with the given set of tags.
pub fn run_tests(tags: &[&str]) -> Result<HashSet<String>> {
  let test_bin = current_exe().context("failed to retrieve test binary")?;
  let args = ["--ignored", "--format=json", "-Zunstable-options"]
    .as_slice()
    .iter()
    .chain(tags);
  let stdout =
    output(&test_bin, args).with_context(|| format!("test `{}` failed", test_bin.display()))?;

  parse_test_output(&stdout)
}


fn parse_test_output(output: &[u8]) -> Result<HashSet<String>> {
  let mut tests = HashSet::new();
  for result in output.lines() {
    let line = result?;
    let line = from_json::<TestLine>(&line)
      .with_context(|| format!("failed to parse JSON test line: `{line}`"))?;

    match line {
      TestLine::Test {
        name,
        event: Event::Ok,
      } => {
        let _inserted = tests.insert(name);
      },
      TestLine::Test {
        name,
        event: Event::Failed,
      } => {
        bail!("test `{name}` ran unsuccessfully")
      },
      TestLine::Test {
        name,
        event: Event::Ignored,
      } => {
        bail!("test `{name}` was ignored")
      },
      _ => (),
    }
  }
  Ok(tests)
}


#[cfg(test)]
mod tests {
  use super::*;

  use maplit::hashset;


  /// Check that we can correctly parse test output lines.
  #[test]
  fn test_output_parsing() {
    let lines = [
      br#"{ "type": "suite", "event": "started", "test_count": 1 }"#.as_slice(),
      br#"{ "type": "suite", "event": "started", "test_count": 0 }"#.as_slice(),
      br#"{ "type": "suite", "event": "ok", "passed": 0, "failed": 0, "ignored": 0, "measured": 0, "filtered_out": 1, "exec_time": 0.000045186 }"#.as_slice(),
      br#"{ "type": "test", "event": "started", "name": "test1::tag1::test" }"#.as_slice(),
      br#"{ "type": "suite", "event": "failed", "passed": 0, "failed": 1, "ignored": 0, "measured": 0, "filtered_out": 2, "exec_time": 0.000191837 }"#.as_slice(),
    ];

    for line in lines {
      let tests = parse_test_output(line).unwrap();
      assert_eq!(tests, hashset! {});
    }

    let line = br#"{ "type": "test", "name": "test1::tag1::test", "event": "ignored" }"#;
    let err = parse_test_output(line).unwrap_err();
    assert_eq!(err.to_string(), "test `test1::tag1::test` was ignored");

    let line = br#"{ "type": "test", "name": "test1::tag1::test", "event": "ok" }"#;
    let tests = parse_test_output(line).unwrap();
    assert_eq!(tests, hashset! {"test1::tag1::test".to_string()});
  }
}
