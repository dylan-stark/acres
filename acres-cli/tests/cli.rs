use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::predicate;
use std::process::Command;

#[test]
fn list_artworks_outputs_json() -> Result<(), Box<dyn std::error::Error>> {
    // Given this artworks list is already in the cache
    let cached_artworks_file = assert_fs::NamedTempFile::new("artworks.json")?;
    cached_artworks_file.write_str(
        r#"{
              "pagination": {
              "total": 128194,
              "limit": 2,
              "offset": 0,
              "total_pages": 64097,
              "current_page": 1,
              "next_url": "https://api.artic.edu/api/v1/artworks?page=2&limit=2&fields=title%2Cdescription"
            },
            "data": [ { "id": 999, "title": "Emergency!" } ]
        }"#,
    )?;

    // When we run the CLI to get artworks list
    let mut cmd = Command::cargo_bin("acres-cli")?;
    cmd.env("ACRES_CACHE_DIR", cached_artworks_file.parent().unwrap())
        .arg("artworks");

    // Then stdout has *only* the list
    let stdout = String::from_utf8(cmd.output()?.stdout)?;
    // And we're able to deserialize it so some valid JSON
    let value: serde_json::Value = serde_json::from_str(&stdout)?;
    assert_eq!(value["data"][0]["id"], 999);
    assert_eq!(value["data"][0]["title"], "Emergency!");

    Ok(())
}

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
