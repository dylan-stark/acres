//! IIIF §2: image and information requests are built from the same base URI. "The scheme,
//! server, prefix and identifier components of the information request must be identical to
//! those for the image request."

use std::{mem::discriminant, str::FromStr};

use iiif::{
    Format, ImageRequest, InformationRequest, Quality, Region, RequestError, Rotation, Size, Uri,
    UriError,
};
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

/// Errors in the base URI come through request parsing wrapped in `InvalidUri`, holding the
/// same variant as parsing the base URI on its own (which `identifier.rs` pins down).
#[rstest]
#[case::raw_at("https://example.org/a@b")]
#[case::raw_non_ascii("https://example.org/☃")]
#[case::bad_escape("https://example.org/abc%zz")]
fn requests_propagate_base_uri_errors(#[case] uri: &str) {
    let expected = uri.parse::<Uri>().expect_err("base URI should be invalid");

    let info = InformationRequest::from_str(&format!("{uri}/info.json"));
    assert!(
        matches!(&info, Err(RequestError::InvalidUri(e)) if discriminant(e) == discriminant(&expected)),
        "expected {expected:?}, got {info:?}"
    );

    let image = ImageRequest::from_str(&format!("{uri}/full/full/0/default.jpg"));
    assert!(
        matches!(&image, Err(RequestError::InvalidUri(e)) if discriminant(e) == discriminant(&expected)),
        "expected {expected:?}, got {image:?}"
    );
}

/// IIIF §2.2: an information request still needs an identifier before `info.json`.
#[test]
fn information_request_without_identifier_is_rejected() {
    let info = InformationRequest::from_str("https://example.org/info.json");
    assert!(
        matches!(
            info,
            Err(RequestError::InvalidUri(
                UriError::MissingIdentifier(_) | UriError::InvalidIdentifier(_)
            ))
        ),
        "{info:?}"
    );
}

/// IIIF §2.1: an image request with too few segments has no base URI before its parameters.
#[test]
fn image_request_without_base_uri_is_rejected() {
    let image = ImageRequest::from_str("full/843,/0/default.jpg");
    assert!(
        matches!(image, Err(RequestError::MissingUri(_))),
        "{image:?}"
    );
}

/// IIIF §2.2: an information request must end in `/info.json`.
#[test]
fn information_request_without_info_json_is_rejected() {
    let info = InformationRequest::from_str("https://example.org/images/12345");
    assert!(
        matches!(info, Err(RequestError::MissingInfoJson(_))),
        "{info:?}"
    );
}

/// A wrapping error's message leaves its cause out and exposes it as `source()` instead, so
/// reporters that walk the chain (eyre, anyhow) print each cause once.
#[test]
fn request_errors_expose_cause_as_source() {
    use std::error::Error as _;

    let err = ImageRequest::from_str("https://example.org/a@b/full/full/0/default.jpg")
        .expect_err("base URI should be invalid");
    let cause = err.source().expect("cause").to_string();
    assert!(cause.starts_with("invalid identifier in URI"), "{cause}");
    assert!(!err.to_string().contains(&cause), "{err}");

    // `iiif::Error` is transparent: same message, same source.
    let wrapped = iiif::Error::from(err);
    assert_eq!(wrapped.to_string(), "invalid base URI in request");
    assert_eq!(wrapped.source().expect("cause").to_string(), cause);
}
