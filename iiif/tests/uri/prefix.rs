//! IIIF §2: "The prefix may contain multiple path segments, delimited by slashes, but all other
//! special characters must be encoded."
//!
//! IIIF §9 lists the special characters (the to-encode set) as `/ ? # [ ] @ %`, plus anything
//! outside US-ASCII.

use iiif::IiifError;
use rstest::rstest;

use crate::{assert_displays_as, assert_round_trips};

/// IIIF §2: the prefix is optional and may have any number of segments.
#[rstest]
#[case::none("https://example.org/abcd1234")]
#[case::one("https://example.org/image-service/abcd1234")]
#[case::many("https://example.org/a/b/c/d/e/abcd1234")]
#[case::aic("https://www.artic.edu/iiif/2/abcd1234")]
fn prefix_segments_round_trip(#[case] s: &str) {
    assert_round_trips(s);
}

/// IIIF §2, §9: encoded special characters, encoded non-ASCII, and RFC 3986 sub-delims and
/// colons are all valid in the prefix.
#[rstest]
#[case::encoded_slash("https://example.org/a%2Fb/abcd1234")]
#[case::encoded_at("https://example.org/a%40b/abcd1234")]
#[case::encoded_query_and_fragment("https://example.org/a%3Fb%23c/abcd1234")]
#[case::encoded_brackets("https://example.org/a%5Bb%5D/abcd1234")]
#[case::encoded_percent("https://example.org/100%25/abcd1234")]
#[case::encoded_non_ascii("https://example.org/caf%C3%A9/abcd1234")]
#[case::encoded_space("https://example.org/a%20b/abcd1234")]
#[case::sub_delims_and_colon("https://example.org/a:b,c;d=e!f$g&h'i(j)k*l+m/abcd1234")]
#[case::unreserved("https://example.org/a-b.c_d~e/abcd1234")]
fn encoded_and_permitted_characters_round_trip(#[case] s: &str) {
    assert_round_trips(s);
}

/// RFC 3986 §5.2.4, §6.2.2.3: `.` and `..` segments are removed.
#[rstest]
#[case::parent("https://example.org/a/../abcd1234", "https://example.org/abcd1234")]
#[case::current(
    "https://example.org/a/./b/abcd1234",
    "https://example.org/a/b/abcd1234"
)]
#[case::parent_past_root("https://example.org/../abcd1234", "https://example.org/abcd1234")]
#[case::nested(
    "https://example.org/a/b/../c/./abcd1234",
    "https://example.org/a/c/abcd1234"
)]
fn dot_segments_are_removed(#[case] s: &str, #[case] expected: &str) {
    assert_displays_as(s, expected);
}

/// IIIF §2, §9: raw special characters in the prefix are rejected.
#[rstest]
#[case::at("https://example.org/a@b/abcd1234")]
#[case::open_bracket("https://example.org/a[b/abcd1234")]
#[case::close_bracket("https://example.org/a]b/abcd1234")]
fn raw_special_characters_are_rejected(#[case] s: &str) {
    assert_err!(s, IiifError::InvalidUriPrefix(_));
}

/// IIIF §9: characters outside US-ASCII must be percent-encoded.
#[rstest]
#[case::latin("https://example.org/café/abcd1234")]
#[case::cjk("https://example.org/画像/abcd1234")]
#[case::emoji("https://example.org/🖼/abcd1234")]
fn raw_non_ascii_is_rejected(#[case] s: &str) {
    assert_err!(s, IiifError::InvalidUriPrefix(_));
}

/// RFC 3986 §2: characters outside the URI character set are not allowed at all.
#[rstest]
#[case::space("https://example.org/a b/abcd1234")]
#[case::double_quote("https://example.org/a\"b/abcd1234")]
#[case::less_than("https://example.org/a<b/abcd1234")]
#[case::greater_than("https://example.org/a>b/abcd1234")]
#[case::backslash("https://example.org/a\\b/abcd1234")]
#[case::caret("https://example.org/a^b/abcd1234")]
#[case::backtick("https://example.org/a`b/abcd1234")]
#[case::open_brace("https://example.org/a{b/abcd1234")]
#[case::pipe("https://example.org/a|b/abcd1234")]
#[case::close_brace("https://example.org/a}b/abcd1234")]
#[case::control("https://example.org/a\u{7f}b/abcd1234")]
fn characters_outside_rfc3986_are_rejected(#[case] s: &str) {
    assert_err!(s, IiifError::InvalidUriPrefix(_));
}

/// IIIF §9: `%` itself must be encoded, so every `%` has to start a valid `%XX` escape
/// (RFC 3986 §2.1).
#[rstest]
#[case::non_hex("https://example.org/a%zz/abcd1234")]
#[case::one_digit("https://example.org/a%4/abcd1234")]
#[case::trailing("https://example.org/a%/abcd1234")]
#[case::half_hex("https://example.org/a%4g/abcd1234")]
#[case::only_percent("https://example.org/%/abcd1234")]
fn malformed_percent_encoding_is_rejected(#[case] s: &str) {
    assert_err!(s, IiifError::InvalidUriPrefix(_));
}

/// IIIF §2: segments are "delimited by slashes". An empty segment is not a segment, and RFC
/// 3986 treats `a//b` and `a/b` as different paths, so it's rejected rather than collapsed.
#[rstest]
#[case::leading("https://example.org//abcd1234")]
#[case::inner("https://example.org/a//b/abcd1234")]
#[case::trailing("https://example.org/a//abcd1234")]
fn empty_segments_are_rejected(#[case] s: &str) {
    assert_err!(s, IiifError::InvalidUriPrefix(_));
}
