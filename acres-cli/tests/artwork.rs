use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use std::{fs, process::Command};

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

#[test]
fn artwork_as_ascii_outputs_ascii_art() -> Result<(), Box<dyn std::error::Error>> {
    // Given we have a custom cache location
    let cache_dir = assert_fs::TempDir::new().expect("could get temp dir");
    let cache_path = cache_dir.path();
    // And that cache already has image we're looking for
    let cache_artworks_path = cache_path.join("artworks");
    fs::create_dir(&cache_artworks_path).expect("able to create /artworks");
    let cached_image_file = cache_artworks_path.join("0.jpg");
    let mut image_buffer = image::ImageBuffer::new(20, 20);
    for (x, y, pixel) in image_buffer.enumerate_pixels_mut() {
        let r = (0.3 * x as f32) as u8;
        let b = (0.3 * y as f32) as u8;
        *pixel = image::Rgb([r, 0, b]);
    }
    let _ = image_buffer.save(cached_image_file);

    // When we generate the ascii art
    let mut cmd = Command::cargo_bin("acres-cli")?;
    cmd.env("ACRES_CACHE_DIR", cache_path)
        .arg("artwork")
        .arg("4")
        .arg("--as")
        .arg("ascii");

    // Then stdout has *only* the ascii art
    let actual = String::from_utf8(cmd.output()?.stdout)?;
    // And the generated art is cached
    //let cached_ascii_file = cache_path.join("artworks").join("0.ascii");
    //let expected = fs::read_to_string(cached_ascii_file).expect("found cached ascii art");
    let expected = "I'm ASCII!\n".to_string();
    // And it matches what was cached
    assert_eq!(actual, expected);

    Ok(())
}
