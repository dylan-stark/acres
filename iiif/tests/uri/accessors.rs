//! IIIF §2: each component of `{scheme}://{server}{/prefix}/{identifier}` can be read back, in
//! canonical, percent-decoded form (§9).

use rstest::rstest;

use crate::ok;

#[rstest]
#[case::plain("https://example.org/abcd1234", "https", "example.org", None, &[], "abcd1234")]
#[case::port_and_prefix(
    "http://127.0.0.1:5555/some/path/to/12345",
    "http",
    "127.0.0.1",
    Some(5555),
    &["some", "path", "to"],
    "12345"
)]
#[case::canonicalized("HTTPS://Example.ORG:443/iiif/2/abc", "https", "example.org", None, &["iiif", "2"], "abc")]
#[case::ipv6("http://[::1]:8080/abcd1234", "http", "[::1]", Some(8080), &[], "abcd1234")]
#[case::decoded(
    "https://example.org/a%2Fb/c%20d/ark:%2F12025%2F654xz321",
    "https",
    "example.org",
    None,
    &["a/b", "c d"],
    "ark:/12025/654xz321"
)]
fn components_are_readable(
    #[case] s: &str,
    #[case] scheme: &str,
    #[case] host: &str,
    #[case] port: Option<u16>,
    #[case] prefix: &[&str],
    #[case] identifier: &str,
) {
    let uri = ok(s);
    assert_eq!(uri.scheme(), scheme);
    assert_eq!(uri.host(), host);
    assert_eq!(uri.port(), port);
    assert_eq!(uri.prefix(), prefix);
    assert_eq!(uri.identifier(), identifier);
}

#[rstest]
#[case("https://example.org/abcd1234")]
#[case("http://[::1]:8080/a%2Fb/c%3Fd%23e")]
#[case("https://example.org/%E2%98%83")]
fn to_url_matches_display(#[case] s: &str) {
    let uri = ok(s);
    assert_eq!(uri.to_url().as_str(), uri.to_string());
}
