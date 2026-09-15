//LICENSE Portions Copyright 2026-2026 PgCentral Foundation, Inc. <contact@pgcentral.org>
//LICENSE
//LICENSE All rights reserved.
//LICENSE
//LICENSE Use of this source code is governed by the MIT license that can be found in the LICENSE file.

use std::process::Command;

/// Path to the freshly-built `cargo-pgrx` binary, provided by cargo at compile time.
const CARGO_PGRX: &str = env!("CARGO_BIN_EXE_cargo-pgrx");

fn run(args: &[&str]) -> (bool, String, String) {
    let output = Command::new(CARGO_PGRX).args(args).output().expect("failed to spawn cargo-pgrx");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (output.status.success(), stdout, stderr)
}

#[test]
fn migrate_help_succeeds() {
    let (success, stdout, stderr) = run(&["pgrx", "migrate", "--help"]);
    assert!(success, "expected --help to succeed, stderr was: {stderr}");
    assert!(stdout.contains("assemble"), "expected assemble subcommand in help output");
}

#[test]
fn migrate_assemble_help_succeeds() {
    let (success, stdout, stderr) = run(&["pgrx", "migrate", "assemble", "--help"]);
    assert!(success, "expected assemble --help to succeed, stderr was: {stderr}");
    assert!(stdout.contains("--prev-version"), "expected --prev-version in help");
    assert!(stdout.contains("--preserve-fragments"), "expected --preserve-fragments in help");
    assert!(stdout.contains("--output-dir"), "expected --output-dir in help");
}

#[test]
fn install_help_has_no_assemble_unreleased() {
    let (success, stdout, stderr) = run(&["pgrx", "install", "--help"]);
    assert!(success, "expected install --help to succeed, stderr was: {stderr}");
    assert!(stdout.contains("--no-assemble-unreleased"), "expected --no-assemble-unreleased in help");
}

#[test]
fn package_help_has_assemble_unreleased() {
    let (success, stdout, stderr) = run(&["pgrx", "package", "--help"]);
    assert!(success, "expected package --help to succeed, stderr was: {stderr}");
    assert!(stdout.contains("--assemble-unreleased"), "expected --assemble-unreleased in help");
}
