use assert_cmd::prelude::*;
use serde_json::{Value, json};
use std::process::Command;

#[tokio::test]
async fn artworks_search_with_q() -> Result<(), Box<dyn std::error::Error>> {
    let body = response_for_q_monet();

    let mock_server = wiremock::MockServer::start().await;
    let mock_uri = format!("{}/api/v1", mock_server.uri());
    wiremock::Mock::given(wiremock::matchers::any())
        .and(wiremock::matchers::path(
            "/api/v1/artworks/search".to_string(),
        ))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(body))
        .expect(1)
        .mount(&mock_server)
        .await;

    // When we run the CLI to get artworks list
    let mut cmd = Command::cargo_bin("acres-cli")?;
    cmd.env("ACRES_BASE_URI", mock_uri)
        .env("ACRES_USE_CACHE", "false") // So it hits wiremock
        .arg("artworks-search")
        .args(["--q", "monet"]);

    // Then stdout has *only* the list
    let stdout = String::from_utf8(cmd.output()?.stdout)?;
    // And we're able to deserialize it so some valid JSON
    let value: serde_json::Value = serde_json::from_str(&stdout)?;
    assert_eq!(value["data"][0]["id"], 16568);
    assert_eq!(
        value["data"][1]["title"],
        "Arrival of the Normandy Train, Gare Saint-Lazare"
    );

    Ok(())
}

fn response_for_q_monet() -> Value {
    json!({
        "preference": null,
        "pagination": {
            "total": 307,
            "limit": 10,
            "offset": 0,
            "total_pages": 31,
            "current_page": 1
        },
        "data": [
            {
                "_score": 226.74677,
                "thumbnail": {
                    "alt_text": "Painting of a pond seen up close spotted with thickly painted pink and white water lilies and a shadow across the top third of the picture.",
                    "width": 8808,
                    "lqip": "data:image/gif;base64,R0lGODlhBQAFAPQAAEZcaFFfdVtqbk9ldFBlcVFocllrcFlrd11rdl9sdFZtf15wcWV0d2R2eGByfmd6eGl6e2t9elZxiGF4kWB4kmJ9kGJ8lWeCkWSAnQAAAAAAAAAAAAAAAAAAAAAAAAAAACH5BAAAAAAALAAAAAAFAAUAAAUVoJBADXI4TLRMWHU9hmRRCjAURBACADs=",
                    "height": 8460
                },
                "api_model": "artworks",
                "is_boosted": true,
                "api_link": "https://api.artic.edu/api/v1/artworks/16568",
                "id": 16568,
                "title": "Water Lilies",
                "timestamp": "2025-01-28T23:26:08-06:00"
            },
            {
                "_score": 210.08162,
                "thumbnail": {
                    "alt_text": "Loosely painted image of an open-air train station. On the right, a parked train gives off an enormous plumb of white smoke, making the scene look as though it were full of clouds. A huddled mass of barely discernible people crowd around the train on both sides of the tracks. Blue, green, and gray tones dominate.",
                    "width": 6786,
                    "lqip": "data:image/gif;base64,R0lGODlhBwAFAPUAADU8QkROS0ZPU0hSVk1YXVFWUlBXXlFaWVNcWFFkV1plVVtjWmBnWmFqXmRrX05ZYFFaYlljbF5qbGNsY2ZydmlzdWRxeGdze2l1fWx3fG16enJ4fH+KioWOkZeam5yjqZ2lqrG1ubS6vwAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACH5BAAAAAAALAAAAAAHAAUAAAYhQIKmYslQDoONp8ORBECi0OfyKEAMmAhAgFhMHA2GIhEEADs=",
                    "height": 5092
                },
                "api_model": "artworks",
                "is_boosted": true,
                "api_link": "https://api.artic.edu/api/v1/artworks/16571",
                "id": 16571,
                "title": "Arrival of the Normandy Train, Gare Saint-Lazare",
                "timestamp": "2025-01-28T23:24:30-06:00"
            },
            {
                "_score": 207.76572,
                "thumbnail": {
                    "alt_text": "Painting composed of short, dense brushstrokes depicts two domed stacks of wheat that cast long shadows on a field. The angled light indicates either a rising or setting sun.",
                    "width": 6884,
                    "lqip": "data:image/gif;base64,R0lGODlhCAAFAPUAAF5eVW1bVm9eVmpjW3RoXXxyV39yXmdsZmhmaXZtbG11eH57eYl5bYR7dHuAf4mDfo6HfpePdpCFeZSOfJ+VdnZ+g4ODgoCHg4iHgo+GgY2MgpmThJeTipaSjaCcmbWnh6qrpKmopqqtrKusrbGxobq4pLu5qq2zsQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACH5BAAAAAAALAAAAAAIAAUAAAYlwJNoFAKRSiZPh7OZRCgfBWJwAAQEBU2D8VgkCAYI5uKoWDKSIAA7",
                    "height": 4068
                },
                "api_model": "artworks",
                "is_boosted": true,
                "api_link": "https://api.artic.edu/api/v1/artworks/64818",
                "id": 64818,
                "title": "Stacks of Wheat (End of Summer)",
                "timestamp": "2025-01-28T23:26:07-06:00"
            }
        ],
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
            "website_url": "http://www.artic.edu"
        }
    })
}
