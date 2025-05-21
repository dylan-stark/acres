use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use std::process::Command;

#[test]
fn artwork_command_outputs_json() -> Result<(), Box<dyn std::error::Error>> {
    // Given this artworks list is already in the cache
    let cached_artwork_file = assert_fs::NamedTempFile::new("artwork.json")?;
    cached_artwork_file.write_str(
        r#"{
            "data": {
                "id": 4,
                "api_model": "artworks",
                "api_link": "https://api.artic.edu/api/v1/artworks/4",
                "is_boosted": false,
                "title": "Priest and Boy",
                "alt_titles": null,
            },
            "info": {
                "license_text": "The `description` field in this response is licensed under a Creative Commons Attribution 4.0 Generic License (CC-By) and the Terms and Conditions of artic.edu. All other data in this response is licensed under a Creative Commons Zero (CC0) 1.0 designation and the Terms and Conditions of artic.edu.",
                "license_links": [
                    "https://creativecommons.org/publicdomain/zero/1.0/",
                    "https://www.artic.edu/terms"
                ],
                "version": "1.13"
            },
            "config": {
                "iiif_url": "https://www.artic.edu/iiif/2",
                "website_url": "https://www.artic.edu"
            }
        }"#,
    )?;

    // When we run the CLI to get artworks list
    let mut cmd = Command::cargo_bin("acres-cli")?;
    cmd.env("ACRES_CACHE_DIR", cached_artwork_file.parent().unwrap())
        .arg("artwork")
        .arg("4");

    // Then stdout has *only* the list
    let stdout = String::from_utf8(cmd.output()?.stdout)?;
    // And we're able to deserialize it so some valid JSON
    let value: serde_json::Value = serde_json::from_str(&stdout)?;
    assert_eq!(value["data"]["id"], 4);

    Ok(())
}
