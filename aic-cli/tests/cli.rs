use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::predicate;
use std::process::Command;

#[test]
fn listing_artworks_outputs_json() -> Result<(), Box<dyn std::error::Error>> {
    let cached_artworks_file = assert_fs::NamedTempFile::new("artworks.json")?;
    cached_artworks_file.write_str(
        r#"{
            "data": [ { "id": 999, "title": "Emergency!" } ]
        }"#,
    )?;

    let mut cmd = Command::cargo_bin("aic-cli")?;
    cmd.env("AIC_CACHE_DIR", cached_artworks_file.parent().unwrap())
        .arg("artworks");

    let stdout = String::from_utf8(cmd.output()?.stdout)?;
    eprintln!("stdout: {:?}", &stdout);
    let value: serde_json::Value = serde_json::from_str(&stdout)?;
    eprintln!("value: {:?}", value);
    assert_eq!(value["data"][0]["id"], 999);
    assert_eq!(value["data"][0]["title"], "Emergency!");

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
