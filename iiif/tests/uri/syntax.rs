//! IIIF §2: the base URI template `{scheme}://{server}{/prefix}/{identifier}`.

use iiif::IiifError;
use rstest::rstest;

use crate::{assert_rejected, assert_round_trips};

/// IIIF §2: well-formed base URIs, with and without a prefix, are already canonical.
#[rstest]
#[case::no_prefix("https://example.org/abcd1234")]
#[case::spec_example("https://example.org/image-service/abcd1234")]
#[case::multi_segment_prefix("https://example.org/some/path/to/12345")]
#[case::http("http://example.org/12345")]
#[case::aic("https://www.artic.edu/iiif/2/3eec80ab-bad4-8b35-1adb-57b7e7cb7adb")]
#[case::uuid_identifier(
    "https://example.org/image-service/abcd1234/1E34750D-38DB-4825-A38A-B60A345E591C"
)]
fn base_uri_round_trips(#[case] s: &str) {
    assert_round_trips(s);
}

/// RFC 3986 §4.3: a base URI must be absolute.
#[rstest]
#[case::empty("")]
#[case::no_scheme("example.org/abcd1234")]
#[case::scheme_relative("//example.org/abcd1234")]
#[case::path_only("/image-service/abcd1234")]
#[case::missing_colon("https//example.org/abcd1234")]
fn relative_references_are_rejected(#[case] s: &str) {
    assert_err!(s, IiifError::InvalidUri(_));
}

/// IIIF §2: the identifier is required.
#[rstest]
#[case::no_path("https://example.org")]
#[case::root_path("https://example.org/")]
#[case::prefix_only_with_slash("https://example.org/image-service/")]
fn missing_identifier_is_rejected(#[case] s: &str) {
    assert_err!(
        s,
        IiifError::MissingIdentifier(_) | IiifError::InvalidIdentifier(_)
    );
}

/// IIIF §5 (3.0) / §5.1 (2.1): the base URI has no trailing slash; a trailing slash would
/// mean an empty identifier.
#[test]
fn trailing_slash_is_rejected() {
    assert_err!(
        "https://example.org/image-service/abcd1234/",
        IiifError::MissingIdentifier(_) | IiifError::InvalidIdentifier(_)
    );
}

/// IIIF §2: parameters are path segments, never query parameters. IIIF §3: `?` and `#` in an
/// identifier must be encoded, so a raw `?` or `#` starts a query or fragment, neither of which
/// belongs in a base URI.
#[rstest]
#[case::query("https://example.org/abcd1234?size=full")]
#[case::empty_query("https://example.org/abcd1234?")]
#[case::fragment("https://example.org/abcd1234#frag")]
#[case::empty_fragment("https://example.org/abcd1234#")]
#[case::query_in_prefix("https://example.org/image-service?x=1/abcd1234")]
#[case::fragment_in_prefix("https://example.org/image-service#x/abcd1234")]
fn query_and_fragment_are_rejected(#[case] s: &str) {
    assert_rejected(s);
}

/// RFC 3986 §2 and Appendix C: whitespace is not part of a URI. Rejected rather than trimmed.
#[rstest]
#[case::leading_space(" https://example.org/abcd1234")]
#[case::trailing_space("https://example.org/abcd1234 ")]
#[case::trailing_newline("https://example.org/abcd1234\n")]
#[case::embedded_tab("https://example.org/ab\tcd1234")]
#[case::embedded_newline("https://example.org/image-\nservice/abcd1234")]
fn whitespace_is_rejected(#[case] s: &str) {
    assert_rejected(s);
}
