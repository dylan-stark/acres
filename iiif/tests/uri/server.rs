//! IIIF §2: `server` is "the host server on which the service resides", optionally with a port.

use iiif::IiifError;
use rstest::rstest;

use crate::{assert_displays_as, assert_rejected, assert_round_trips};

/// IIIF §2: hosts may be registered names or IP literals, with or without a port.
#[rstest]
#[case::reg_name("https://example.org/abcd1234")]
#[case::subdomain("https://www.artic.edu/iiif/2/abcd1234")]
#[case::non_default_port("http://example.org:5555/12345")]
#[case::ipv4("http://127.0.0.1/abcd1234")]
#[case::ipv4_with_port("http://127.0.0.1:5555/some/path/to/12345")]
#[case::ipv6("http://[::1]/abcd1234")]
#[case::ipv6_with_port("http://[::1]:8080/abcd1234")]
#[case::ipv6_full("http://[2001:db8::1]/abcd1234")]
#[case::localhost("http://localhost:8099/abcd1234")]
#[case::hyphenated("https://iiif-server.example.org/abcd1234")]
fn hosts_are_accepted(#[case] s: &str) {
    assert_round_trips(s);
}

/// RFC 3986 §3.2.2, §6.2.2.1: hosts are case-insensitive; the canonical form is lowercase.
/// RFC 5952 §4.3: the same goes for IPv6 hex digits.
#[rstest]
#[case::upper("https://EXAMPLE.ORG/abcd1234", "https://example.org/abcd1234")]
#[case::mixed(
    "https://Www.Artic.Edu/iiif/2/abcd1234",
    "https://www.artic.edu/iiif/2/abcd1234"
)]
#[case::ipv6_upper("http://[2001:DB8::1]/abcd1234", "http://[2001:db8::1]/abcd1234")]
fn host_is_lowercased(#[case] s: &str, #[case] expected: &str) {
    assert_displays_as(s, expected);
}

/// RFC 3986 §6.2.3: the scheme's default port and an empty port are omitted from the canonical
/// form; any other port is kept.
#[rstest]
#[case::http_80("http://example.org:80/abcd1234", "http://example.org/abcd1234")]
#[case::https_443("https://example.org:443/abcd1234", "https://example.org/abcd1234")]
#[case::ipv4_80(
    "http://127.0.0.1:80/some/path/to/12345",
    "http://127.0.0.1/some/path/to/12345"
)]
#[case::empty_port("http://example.org:/abcd1234", "http://example.org/abcd1234")]
#[case::https_80("https://example.org:80/abcd1234", "https://example.org:80/abcd1234")]
#[case::http_443("http://example.org:443/abcd1234", "http://example.org:443/abcd1234")]
fn ports_are_normalized(#[case] s: &str, #[case] expected: &str) {
    assert_displays_as(s, expected);
}

/// RFC 3986 §3.2: the authority must be a valid host and port.
#[rstest]
#[case::empty_host("http:///abcd1234")]
#[case::bare_scheme("http:")]
#[case::bare_authority("http://")]
#[case::invalid_ipv4("http://0.0.0.999/abcd1234")]
#[case::port_too_large("http://example.com:65536/abcd1234")]
#[case::port_overflow("http://example.com:999999999999/abcd1234")]
#[case::non_numeric_port("http://example.com:port/abcd1234")]
#[case::space_in_host("http://exa mple.org/abcd1234")]
#[case::unclosed_ipv6("http://[::1/abcd1234")]
#[case::invalid_ipv6("http://[::g]/abcd1234")]
fn invalid_authority_is_rejected(#[case] s: &str) {
    assert_err!(s, IiifError::InvalidUri(_) | IiifError::MissingServer(_));
}

/// IIIF §2: `server` is a host and an optional port. RFC 3986 §3.2.1 userinfo isn't part of it,
/// and RFC 3986 §7.5 deprecates credentials in URIs.
#[rstest]
#[case::user_and_password("https://user:pw@example.org/abcd1234")]
#[case::user("https://user@example.org/abcd1234")]
#[case::empty_userinfo("https://@example.org/abcd1234")]
#[case::empty_password("https://user:@example.org/abcd1234")]
fn userinfo_is_rejected(#[case] s: &str) {
    assert_rejected(s);
}
