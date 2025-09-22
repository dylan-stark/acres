use assert_cmd::prelude::*;
use serde_json::json;
use std::process::Command;

#[tokio::test]
async fn artworks_command_outputs_json() -> Result<(), Box<dyn std::error::Error>> {
    let id = 999;
    let title = "Emergency!";
    let body = json!({
          "pagination": {
          "total": 128194,
          "limit": 2,
          "offset": 0,
          "total_pages": 64097,
          "current_page": 1,
          "next_url": "https://api.artic.edu/api/v1/artworks?page=2&limit=2&fields=title%2Cdescription"
        },
        "data": [ { "id": id, "title": title } ]
    });

    let mock_server = wiremock::MockServer::start().await;
    let mock_uri = format!("{}/api/v1", mock_server.uri());
    wiremock::Mock::given(wiremock::matchers::any())
        .and(wiremock::matchers::path("/api/v1/artworks".to_string()))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(body))
        .expect(1)
        .mount(&mock_server)
        .await;

    // When we run the CLI to get artworks list
    let mut cmd = Command::cargo_bin("acres-cli")?;
    cmd.env("ACRES_BASE_URI", mock_uri)
        .env("ACRES_USE_CACHE", "false") // So it hits wiremock
        .arg("artworks");

    // Then stdout has *only* the list
    let stdout = String::from_utf8(cmd.output()?.stdout)?;
    // And we're able to deserialize it so some valid JSON
    let value: serde_json::Value = serde_json::from_str(&stdout)?;
    assert_eq!(value["data"][0]["id"], id);
    assert_eq!(value["data"][0]["title"], title);

    Ok(())
}
