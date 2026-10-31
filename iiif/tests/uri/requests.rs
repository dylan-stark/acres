//! IIIF §2: image and information requests are built from the same base URI. "The scheme,
//! server, prefix and identifier components of the information request must be identical to
//! those for the image request."

use std::str::FromStr;

use iiif::{Format, IiifError, ImageRequest, InformationRequest, Quality, Region, Rotation, Size};
use rstest::rstest;

use crate::ok;

fn image_request(uri: &str) -> ImageRequest {
    ImageRequest::new(
        ok(uri),
        Region::Full,
        Size::Width(843),
        Rotation::default(),
        Quality::Default,
        Format::Jpg,
    )
}

/// IIIF §2.2 (3.0) / §2.2 (2.1): `{scheme}://{server}{/prefix}/{identifier}/info.json`.
#[rstest]
#[case::aic("https://www.artic.edu/iiif/2/3eec80ab-bad4-8b35-1adb-57b7e7cb7adb")]
#[case::ark("https://example.org/image-service/ark:%2F12025%2F654xz321")]
#[case::urn_sici("https://example.org/urn:sici:1046-8188(199501)13:1%253C69:FTTHBI%253E2.0.TX;2-4")]
#[case::url_identifier("http://127.0.0.1:8099/http:%2F%2Fexample.com%2F%3F54%23a")]
fn information_request_embeds_base_uri(#[case] uri: &str) {
    let request = InformationRequest::new(ok(uri));
    let url = format!("{uri}/info.json");
    assert_eq!(request.to_string(), url);
    assert_eq!(InformationRequest::from_str(&url).unwrap(), request);
}

/// IIIF §2.1:
/// `{scheme}://{server}{/prefix}/{identifier}/{region}/{size}/{rotation}/{quality}.{format}`.
#[rstest]
#[case::aic("https://www.artic.edu/iiif/2/3eec80ab-bad4-8b35-1adb-57b7e7cb7adb")]
#[case::ark("https://example.org/image-service/ark:%2F12025%2F654xz321")]
#[case::urn_sici("https://example.org/urn:sici:1046-8188(199501)13:1%253C69:FTTHBI%253E2.0.TX;2-4")]
#[case::url_identifier("http://127.0.0.1:8099/http:%2F%2Fexample.com%2F%3F54%23a")]
fn image_request_embeds_base_uri(#[case] uri: &str) {
    let request = image_request(uri);
    let url = format!("{uri}/full/843,/0/default.jpg");
    assert_eq!(request.to_string(), url);
    assert_eq!(ImageRequest::from_str(&url).unwrap(), request);
}

/// The base URI inside a request is normalized just like a standalone one.
#[test]
fn requests_normalize_the_base_uri() {
    let info: InformationRequest = "HTTPS://Example.org:443/iiif/ark:%2f12025/info.json"
        .parse()
        .unwrap();
    assert_eq!(
        info.to_string(),
        "https://example.org/iiif/ark:%2F12025/info.json"
    );

    let image: ImageRequest = "HTTPS://Example.org:443/iiif/ark:%2f12025/full/843,/0/default.jpg"
        .parse()
        .unwrap();
    assert_eq!(
        image.to_string(),
        "https://example.org/iiif/ark:%2F12025/full/843,/0/default.jpg"
    );
}

/// Errors in the base URI come through request parsing unchanged.
#[rstest]
#[case::raw_at("https://example.org/a@b")]
#[case::raw_non_ascii("https://example.org/☃")]
#[case::bad_escape("https://example.org/abc%zz")]
fn requests_propagate_identifier_errors(#[case] uri: &str) {
    let info = InformationRequest::from_str(&format!("{uri}/info.json"));
    assert!(
        matches!(info, Err(IiifError::InvalidIdentifier(_))),
        "{info:?}"
    );

    let image = ImageRequest::from_str(&format!("{uri}/full/full/0/default.jpg"));
    assert!(
        matches!(image, Err(IiifError::InvalidIdentifier(_))),
        "{image:?}"
    );
}

/// IIIF §2.2: an information request still needs an identifier before `info.json`.
#[test]
fn information_request_without_identifier_is_rejected() {
    let info = InformationRequest::from_str("https://example.org/info.json");
    assert!(
        matches!(
            info,
            Err(IiifError::MissingIdentifier(_) | IiifError::InvalidIdentifier(_))
        ),
        "{info:?}"
    );
}
