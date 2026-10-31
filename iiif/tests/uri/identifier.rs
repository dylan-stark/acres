//! IIIF §3 Identifier and §9 URI Encoding and Decoding.
//!
//! §3: "The API places no restrictions on the form of the identifiers ... All special characters
//! (e.g. ? or #) must be URI encoded ... slashes ('/') in the identifier must be URI encoded."
//!
//! §9: to-encode = "/" / "?" / "#" / "[" / "]" / "@" / "%", plus any character outside
//! US-ASCII.

use iiif::IiifError;
use rstest::rstest;

use crate::{assert_round_trips, ok};

const BASE: &str = "https://example.org/image-service/";

/// IIIF §9: every row of the spec's identifier-encoding table, already encoded, round-trips.
#[rstest]
#[case::plain("id1")]
#[case::alphanumeric("bb157hs6068")]
#[case::ark("ark:%2F12025%2F654xz321")]
#[case::urn("urn:foo:a123,456")]
#[case::urn_sici("urn:sici:1046-8188(199501)13:1%253C69:FTTHBI%253E2.0.TX;2-4")]
#[case::url("http:%2F%2Fexample.com%2F%3F54%23a")]
fn spec_examples_round_trip(#[case] encoded: &str) {
    assert_round_trips(&format!("{BASE}{encoded}"));
}

/// IIIF §3: an encoded `/` is part of the identifier, not a path separator.
#[test]
fn encoded_slash_is_not_a_separator() {
    let encoded = ok("https://example.org/a%2Fb");
    let separated = ok("https://example.org/a/b");
    assert_ne!(encoded, separated);
    assert_eq!(encoded.to_string(), "https://example.org/a%2Fb");
    assert_eq!(separated.to_string(), "https://example.org/a/b");
}

/// IIIF §9: the to-encode set, encoded non-ASCII, and characters RFC 3986 forbids raw all
/// round-trip in encoded form.
#[rstest]
#[case::percent("100%25")]
#[case::all_specials("%2F%3F%23%5B%5D%40%25")]
#[case::snowman("%E2%98%83")]
#[case::latin("caf%C3%A9")]
#[case::space("a%20b")]
#[case::double_quote("a%22b")]
#[case::angle_brackets("a%3Cb%3E")]
#[case::backslash("a%5Cb")]
#[case::braces_and_pipe("a%7Bb%7Cc%7D")]
#[case::caret_and_backtick("a%5Eb%60c")]
#[case::control("a%00b%7F")]
fn encoded_identifiers_round_trip(#[case] encoded: &str) {
    assert_round_trips(&format!("{BASE}{encoded}"));
}

/// IIIF §9: characters outside the to-encode set are not encoded, as long as RFC 3986 allows
/// them raw (unreserved, sub-delims, ":").
#[rstest]
#[case::unreserved("aZ09-._~")]
#[case::sub_delims("!$&'()*+,;=")]
#[case::colon("ark:12025")]
#[case::dots_inside(".hidden")]
#[case::three_dots("...")]
#[case::uuid("3eec80ab-bad4-8b35-1adb-57b7e7cb7adb")]
fn permitted_raw_characters_round_trip(#[case] encoded: &str) {
    assert_round_trips(&format!("{BASE}{encoded}"));
}

/// IIIF §3, §9: raw special characters in the identifier are rejected. (Raw `/`, `?` and `#`
/// can't be detected here: they change how the URI splits up. See `syntax.rs` and `prefix.rs`.)
#[rstest]
#[case::at("a@b")]
#[case::open_bracket("a[b")]
#[case::close_bracket("a]b")]
#[case::both_brackets("[0]")]
fn raw_special_characters_are_rejected(#[case] raw: &str) {
    assert_err!(&format!("{BASE}{raw}"), IiifError::InvalidIdentifier(_));
}

/// IIIF §9: characters outside US-ASCII must be percent-encoded.
#[rstest]
#[case::latin("café")]
#[case::snowman("☃")]
#[case::cjk("画像")]
#[case::rtl("صورة")]
#[case::combining("e\u{301}")]
fn raw_non_ascii_is_rejected(#[case] raw: &str) {
    assert_err!(&format!("{BASE}{raw}"), IiifError::InvalidIdentifier(_));
}

/// RFC 3986 §2: characters outside the URI character set are not allowed at all.
#[rstest]
#[case::space("a b")]
#[case::double_quote("a\"b")]
#[case::less_than("a<b")]
#[case::greater_than("a>b")]
#[case::backslash("a\\b")]
#[case::caret("a^b")]
#[case::backtick("a`b")]
#[case::open_brace("a{b")]
#[case::pipe("a|b")]
#[case::close_brace("a}b")]
#[case::nul("a\0b")]
#[case::delete("a\u{7f}b")]
fn characters_outside_rfc3986_are_rejected(#[case] raw: &str) {
    assert_err!(&format!("{BASE}{raw}"), IiifError::InvalidIdentifier(_));
}

/// IIIF §9: `%` must be encoded, so every `%` has to start a valid `%XX` escape (RFC 3986
/// §2.1).
#[rstest]
#[case::non_hex("abc%zz")]
#[case::one_digit("abc%4")]
#[case::trailing("abc%")]
#[case::half_hex("abc%4g")]
#[case::only_percent("%")]
#[case::unencoded_percent_literal("100%")]
fn malformed_percent_encoding_is_rejected(#[case] raw: &str) {
    assert_err!(&format!("{BASE}{raw}"), IiifError::InvalidIdentifier(_));
}

/// IIIF §9: percent-encoding encodes UTF-8 octets, so the decoded identifier has to be valid
/// UTF-8.
#[rstest]
#[case::invalid_byte("%FF")]
#[case::truncated_sequence("%C3")]
#[case::lone_continuation("%80")]
#[case::overlong("%C0%AF")]
#[case::surrogate("%ED%A0%80")]
fn invalid_utf8_is_rejected(#[case] encoded: &str) {
    assert_err!(&format!("{BASE}{encoded}"), IiifError::InvalidIdentifier(_));
}

/// RFC 3986 §5.2.4, §2.3: `.` and `..` segments, encoded or not, are removed during resolution,
/// so no URI can address an identifier spelled that way.
#[rstest]
#[case::dot(".")]
#[case::dot_dot("..")]
#[case::encoded_dot("%2E")]
#[case::encoded_dot_dot("%2E%2E")]
#[case::mixed_dot_dot(".%2E")]
#[case::lowercase_encoded_dot("%2e")]
fn dot_segment_identifiers_are_rejected(#[case] raw: &str) {
    assert_err!(
        &format!("{BASE}{raw}"),
        IiifError::InvalidIdentifier(_) | IiifError::MissingIdentifier(_)
    );
}
