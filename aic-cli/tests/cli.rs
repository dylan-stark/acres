use assert_cmd::prelude::*;
use predicates::prelude::predicate;
use std::process::Command;

#[test]
fn artworks_resource() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("aic-cli")?;
    cmd.arg("artworks");

    cmd.assert().success();
    Ok(())
}

#[test]
fn unsupported_resource() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("aic-cli")?;
    cmd.arg("artists");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("invalid value 'artists'"))
        .stderr(predicate::str::contains("possible values: artworks"));
    Ok(())
}
