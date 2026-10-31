//! RFC 3986 §6.2.2 syntax-based normalization: different spellings of the same base URI parse
//! to equal values and display in a single canonical form. IIIF §9 defines the identifier by its
//! decoded value, so encoding a character the to-encode set doesn't require changes nothing.

use std::hash::{DefaultHasher, Hash, Hasher};

use iiif::Uri;
use rstest::rstest;

use crate::ok;

fn hash(uri: &Uri) -> u64 {
    let mut hasher = DefaultHasher::new();
    uri.hash(&mut hasher);
    hasher.finish()
}

/// Each `variant` is equivalent to the `canonical` spelling: the two compare equal, hash equal,
/// and display as `canonical`.
#[rstest]
// RFC 3986 §6.2.2.1: scheme and host are case-insensitive.
#[case::scheme_case("HTTPS://example.org/abcd1234", "https://example.org/abcd1234")]
#[case::host_case("https://Example.ORG/abcd1234", "https://example.org/abcd1234")]
// RFC 3986 §6.2.3: default ports.
#[case::default_port("https://example.org:443/abcd1234", "https://example.org/abcd1234")]
// RFC 3986 §2.1, §6.2.2.1: hex digits in escapes are uppercase.
#[case::lowercase_hex_identifier(
    "https://example.org/ark:%2f12025%2f654xz321",
    "https://example.org/ark:%2F12025%2F654xz321"
)]
#[case::lowercase_hex_non_ascii("https://example.org/%e2%98%83", "https://example.org/%E2%98%83")]
#[case::lowercase_hex_prefix(
    "https://example.org/a%2fb/abcd1234",
    "https://example.org/a%2Fb/abcd1234"
)]
// RFC 3986 §2.3, §6.2.2.2: encoded unreserved characters are decoded.
#[case::encoded_digit("https://example.org/abc%31", "https://example.org/abc1")]
#[case::encoded_letter("https://example.org/%61bc", "https://example.org/abc")]
#[case::encoded_tilde("https://example.org/%7Euser", "https://example.org/~user")]
#[case::encoded_hyphen_in_prefix(
    "https://example.org/image%2Dservice/abcd1234",
    "https://example.org/image-service/abcd1234"
)]
// IIIF §9: identifier characters outside the to-encode set are canonically left raw.
#[case::encoded_colon("https://example.org/ark%3A12025", "https://example.org/ark:12025")]
#[case::encoded_comma(
    "https://example.org/urn:foo:a123%2C456",
    "https://example.org/urn:foo:a123,456"
)]
#[case::encoded_semicolon("https://example.org/a%3Bb", "https://example.org/a;b")]
#[case::encoded_parens("https://example.org/%28199501%29", "https://example.org/(199501)")]
// RFC 3986 §5.2.4: dot segments in the prefix.
#[case::dot_segments("https://example.org/a/../abcd1234", "https://example.org/abcd1234")]
#[case::everything_at_once(
    "HTTPS://EXAMPLE.org:443/a/./image%2dservice/ark%3a%2f12025",
    "https://example.org/a/image-service/ark:%2F12025"
)]
fn equivalent_spellings_are_equal(#[case] variant: &str, #[case] canonical: &str) {
    let (variant, canonical_uri) = (ok(variant), ok(canonical));
    assert_eq!(variant, canonical_uri);
    assert_eq!(hash(&variant), hash(&canonical_uri));
    assert_eq!(variant.to_string(), canonical);
}

/// Spellings that differ in a meaningful way stay distinct.
#[rstest]
// RFC 3986 §6.2.2.1: only scheme and host are case-insensitive; IIIF §3 places no
// restriction on identifiers, so their case matters.
#[case::identifier_case("https://example.org/ABCD1234", "https://example.org/abcd1234")]
#[case::prefix_case(
    "https://example.org/IIIF/abcd1234",
    "https://example.org/iiif/abcd1234"
)]
// IIIF §3: an encoded slash belongs to the identifier.
#[case::encoded_slash("https://example.org/a%2Fb", "https://example.org/a/b")]
// IIIF §9: `%` is in the to-encode set, so `%25` and `%` differ.
#[case::double_encoding("https://example.org/a%252F", "https://example.org/a%2F")]
#[case::scheme("http://example.org/abcd1234", "https://example.org/abcd1234")]
#[case::port("http://example.org:8080/abcd1234", "http://example.org/abcd1234")]
#[case::host("https://example.com/abcd1234", "https://example.org/abcd1234")]
#[case::prefix("https://example.org/a/abcd1234", "https://example.org/abcd1234")]
fn distinct_spellings_are_not_equal(#[case] a: &str, #[case] b: &str) {
    assert_ne!(ok(a), ok(b));
}
