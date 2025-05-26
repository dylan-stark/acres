use assert_cmd::prelude::*;
use eyre::Context;
use serde_json::json;
use std::{fs, process::Command};

#[test]
fn artwork_command_outputs_json() -> Result<(), Box<dyn std::error::Error>> {
    // Given we have a custom cache location
    let cache_dir = assert_fs::TempDir::new().expect("could get temp dir");
    let cache_path = cache_dir.path();
    // And that cache already has the artwork JSON
    put_dummy_artwork_in_cache(cache_path);

    // When we run the CLI to get artworks list
    let mut cmd = Command::cargo_bin("acres-cli")?;
    cmd.env("ACRES_CACHE_DIR", cache_path)
        .arg("artwork")
        .arg("0");

    // Then stdout has *only* the list
    let output = cmd.output()?;
    let stdout = String::from_utf8(output.stdout)?;
    // And we're able to deserialize it so some valid JSON
    let value: serde_json::Value =
        serde_json::from_str(&stdout).context(format!("failed to parse stdout: '{}'", &stdout))?;
    assert_eq!(value["data"]["id"].as_i64(), Some(0));

    Ok(())
}

#[test]
fn artwork_as_ascii_outputs_ascii_art() -> Result<(), Box<dyn std::error::Error>> {
    // Given we have a custom cache location
    let cache_dir = assert_fs::TempDir::new().expect("could get temp dir");
    let cache_path = cache_dir.path();
    // And that cache already has the artwork JSON
    put_dummy_artwork_in_cache(cache_path);
    // And that cache already has the image we're looking for
    let cache_artworks_path = cache_path.join("images");
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
        .arg("0")
        .arg("--as")
        .arg("ascii");

    // Then stdout has *only* the ascii art
    let output = cmd.output()?;
    let actual = String::from_utf8(output.stdout)?;
    // And the generated art is cached
    let cached_ascii_file = cache_path.join("ascii").join("0.ascii");
    let expected = fs::read_to_string(cached_ascii_file).expect("found cached ascii art");
    // And it matches what was cached
    assert_eq!(actual, expected);

    Ok(())
}

fn put_dummy_artwork_in_cache(cache_path: &std::path::Path) {
    let filename = "artwork.0.json".to_string();
    let json = json!({
        "data": { "id": 0, "image_id": "0" }
    })
    .to_string();
    fs::write(cache_path.join(filename), json).expect("can write to cache");
}
