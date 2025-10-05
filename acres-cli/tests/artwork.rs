use assert_cmd::prelude::*;
use eyre::Context;
use serde_json::json;
use std::process::Command;

#[tokio::test]
async fn artwork_command_outputs_json() -> Result<(), Box<dyn std::error::Error>> {
    let id = 42;
    let body = json!({"data":{"id": id}});

    let mock_server = wiremock::MockServer::start().await;
    let mock_uri = format!("{}/api/v1", mock_server.uri());
    wiremock::Mock::given(wiremock::matchers::any())
        .and(wiremock::matchers::path(format!("/api/v1/artworks/{}", id)))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(body))
        .expect(1)
        .mount(&mock_server)
        .await;

    // When we run the CLI to get artworks list
    let mut cmd = Command::cargo_bin("acres-cli")?;
    cmd.env("ACRES_BASE_URI", mock_uri)
        .env("ACRES_USE_CACHE", "false") // So it hits wiremock
        .arg("artwork")
        .arg(id.to_string());

    // Then stdout has *only* that JSON
    let output = cmd.output()?;
    println!("output={:?}", output);
    let stdout = String::from_utf8(output.stdout)?;
    // And we're able to deserialize it so some valid JSON
    let value: serde_json::Value =
        serde_json::from_str(&stdout).context(format!("failed to parse stdout: '{}'", &stdout))?;
    assert_eq!(value["data"]["id"].as_i64(), Some(id));

    Ok(())
}

#[tokio::test]
async fn artwork_manifest_command_outputs_json() -> Result<(), Box<dyn std::error::Error>> {
    let id = 42;
    let body = json!(
    {
        "@context": "http://iiif.io/api/presentation/2/context.json",
        "@id": "https://api.artic.edu/api/v1/artworks/4/manifest.json",
        "@type": "sc:Manifest",
        "label": "Priest and Boy",
        "description": [
            {
                "value": "",
                "language": "en"
            }
        ],
        "metadata": [
            {
                "label": "Artist / Maker",
                "value": "Lawrence Carmichael Earle\nAmerican, 1845-1921"
            },
            {
                "label": "Medium",
                "value": "Watercolor over graphite on cream wove paper"
            },
        ]
    });

    let mock_server = wiremock::MockServer::start().await;
    let mock_uri = format!("{}/api/v1", mock_server.uri());
    wiremock::Mock::given(wiremock::matchers::any())
        .and(wiremock::matchers::path(format!(
            "/api/v1/artworks/{}/manifest",
            id
        )))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(body))
        .expect(1)
        .mount(&mock_server)
        .await;

    // When we run the CLI to get artworks list
    let mut cmd = Command::cargo_bin("acres-cli")?;
    cmd.env("ACRES_BASE_URI", mock_uri)
        .env("ACRES_USE_CACHE", "false") // So it hits wiremock
        .arg("artwork-manifest")
        .arg(id.to_string());

    // Then stdout has *only* that JSON
    let output = cmd.output()?;
    let stdout = String::from_utf8(output.stdout)?;
    // And we're able to deserialize it so some valid JSON
    let value: serde_json::Value =
        serde_json::from_str(&stdout).context(format!("failed to parse stdout: '{}'", &stdout))?;
    assert_eq!(value["@type"], "sc:Manifest");

    Ok(())
}

//#[test]
//fn artwork_as_ascii_outputs_ascii_art() -> Result<(), Box<dyn std::error::Error>> {
//    // Given we have a custom cache location
//    let cache_dir = assert_fs::TempDir::new().expect("could get temp dir");
//    let cache_path = cache_dir.path();
//    // And that cache already has the artwork JSON
//    put_dummy_artwork_in_cache(cache_path);
//    // And that cache already has the image we're looking for
//    let cache_artworks_path = cache_path.join("images");
//    fs::create_dir(&cache_artworks_path).expect("able to create /artworks");
//    let cached_image_file = cache_artworks_path.join("0.jpg");
//    let mut image_buffer = image::ImageBuffer::new(20, 20);
//    for (x, y, pixel) in image_buffer.enumerate_pixels_mut() {
//        let r = (0.3 * x as f32) as u8;
//        let b = (0.3 * y as f32) as u8;
//        *pixel = image::Rgb([r, 0, b]);
//    }
//    let _ = image_buffer.save(cached_image_file);
//
//    // When we generate the ascii art
//    let mut cmd = Command::cargo_bin("acres-cli")?;
//    cmd.env("ACRES_CACHE_DIR", cache_path)
//        .arg("artwork")
//        .arg("0")
//        .arg("--as")
//        .arg("ascii");
//
//    // Then stdout has *only* the ascii art
//    let output = cmd.output()?;
//    let mut actual = String::from_utf8(output.stdout)?;
//    actual.pop();
//    // And the generated art is cached
//    let cached_ascii_file = cache_path.join("ascii").join("0.80.ascii");
//    let expected = fs::read_to_string(cached_ascii_file).expect("found cached ascii art");
//    // And it matches what was cached
//    assert_eq!(actual, expected);
//
//    Ok(())
//}
