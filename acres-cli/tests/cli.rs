use assert_cmd::prelude::*;
use predicates::prelude::predicate;
use std::process::Command;

#[test]
fn requires_subcommand() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("acres-cli")?;

    cmd.assert().failure().stderr(predicate::str::contains(
        "requires a subcommand but one was not provided",
    ));
    Ok(())
}

#[test]
fn unsupported_subcommand() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("acres-cli")?;
    cmd.arg("artists");

    cmd.assert().failure().stderr(predicate::str::contains(
        "unrecognized subcommand 'artists'",
    ));
    Ok(())
}
