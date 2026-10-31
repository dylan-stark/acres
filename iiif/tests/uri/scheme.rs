//! IIIF §2: `scheme` is either `http` or `https`.

use iiif::IiifError;
use rstest::rstest;

use crate::{assert_displays_as, assert_round_trips};

/// IIIF §2: "Indicates the use of the HTTP or HTTPS protocol in calling the service."
#[rstest]
#[case::http("http://example.org/abcd1234")]
#[case::https("https://example.org/abcd1234")]
fn http_and_https_are_accepted(#[case] s: &str) {
    assert_round_trips(s);
}

/// RFC 3986 §3.1, §6.2.2.1: schemes are case-insensitive; the canonical form is lowercase.
#[rstest]
#[case::upper_https("HTTPS://example.org/abcd1234", "https://example.org/abcd1234")]
#[case::upper_http("HTTP://example.org/abcd1234", "http://example.org/abcd1234")]
#[case::mixed("Https://example.org/abcd1234", "https://example.org/abcd1234")]
fn scheme_is_lowercased(#[case] s: &str, #[case] expected: &str) {
    assert_displays_as(s, expected);
}

/// IIIF §2: any other scheme with a server and path is rejected as a bad scheme.
#[rstest]
#[case::ftp("ftp://example.org/abcd1234")]
#[case::ws("ws://example.org/abcd1234")]
#[case::wss("wss://example.org/abcd1234")]
#[case::gopher("gopher://example.org/abcd1234")]
#[case::file_with_host("file://example.org/abcd1234")]
#[case::file_without_host("file:///abcd1234")]
#[case::lookalike("httpx://example.org/abcd1234")]
#[case::shttp("shttp://example.org/abcd1234")]
fn other_schemes_are_rejected(#[case] s: &str) {
    assert_err!(s, IiifError::InvalidScheme(_));
}

/// IIIF §2: schemes without a hierarchical `//server` part are rejected.
#[rstest]
#[case::mailto("mailto:someone@example.org")]
#[case::urn("urn:example:abcd1234")]
#[case::data("data:,abcd1234")]
fn non_hierarchical_schemes_are_rejected(#[case] s: &str) {
    assert_err!(
        s,
        IiifError::InvalidScheme(_) | IiifError::InvalidUri(_) | IiifError::MissingServer(_)
    );
}

/// RFC 3986 §3: `https:example.org/x` has no authority, so it has no server. Rejected rather
/// than fixed up the way WHATWG URL parsers do.
#[rstest]
#[case::no_slashes("https:example.org/abcd1234")]
#[case::one_slash("https:/example.org/abcd1234")]
#[case::three_slashes("https:///example.org/abcd1234")]
#[case::backslashes("https:\\\\example.org\\abcd1234")]
fn missing_authority_is_rejected(#[case] s: &str) {
    assert_err!(s, IiifError::InvalidUri(_) | IiifError::MissingServer(_));
}
