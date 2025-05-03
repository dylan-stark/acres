use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::predicate;
use std::process::Command;

#[test]
fn listing_artworks_outputs_json() -> Result<(), Box<dyn std::error::Error>> {
    let cached_artworks_file = assert_fs::NamedTempFile::new("artworks.json")?;
    cached_artworks_file.write_str("{}")?;

    let mut cmd = Command::cargo_bin("aic-cli")?;
    cmd.env("AIC_CACHE_DIR", cached_artworks_file.parent().unwrap())
        .arg("artworks");

    let stdout = String::from_utf8(cmd.output()?.stdout)?;

    serde_json::from_str::<serde_json::Value>(&stdout)?;
    Ok(())
}

#[test]
fn no_resource() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("aic-cli")?;

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains(
            "the following required arguments were not provided:",
        ))
        .stderr(predicate::str::contains("<RESOURCE>"));
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
