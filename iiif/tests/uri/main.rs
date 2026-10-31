//! Specification tests for [`iiif::Uri`], the IIIF base URI.
//!
//! The base URI is `{scheme}://{server}{/prefix}/{identifier}`. Image API 2.1 and 3.0 define it
//! identically, so these tests apply to both:
//!
//! - IIIF Image API §2 URI Syntax, §3 Identifier, §9 URI Encoding and Decoding:
//!   <https://iiif.io/api/image/2.1/#uri-syntax>, <https://iiif.io/api/image/3.0/#2-uri-syntax>
//! - RFC 3986 (URI generic syntax) for character classes and normalization:
//!   <https://www.rfc-editor.org/rfc/rfc3986>
//!
//! Policy: input that violates either spec is rejected, never repaired. Input that is valid but
//! not canonical (letter case, default ports, needless percent-encoding, dot segments) is
//! accepted and displayed in canonical form.
//!
//! Errors are matched by variant, not message, so rewording errors does not break tests. Where
//! the specs don't determine which variant applies, the pattern accepts each reasonable one.

use iiif::Uri;

/// Asserts that parsing `$input` as a [`Uri`] fails with an error matching `$pattern`.
macro_rules! assert_err {
    ($input:expr, $pattern:pat $(,)?) => {{
        let input: &str = $input;
        match input.parse::<::iiif::Uri>() {
            Err($pattern) => {}
            other => panic!(
                "{input:?}: expected Err({}), got {other:?}",
                stringify!($pattern)
            ),
        }
    }};
}

mod equivalence;
mod identifier;
mod prefix;
mod properties;
mod requests;
mod scheme;
mod server;
mod syntax;

/// Parses `s`, panicking with the input and error on failure.
#[track_caller]
fn ok(s: &str) -> Uri {
    s.parse()
        .unwrap_or_else(|e| panic!("{s:?} should parse, got {e:?}"))
}

/// Asserts that `s` parses and displays as `expected`.
#[track_caller]
fn assert_displays_as(s: &str, expected: &str) {
    assert_eq!(ok(s).to_string(), expected, "display of {s:?}");
}

/// Asserts that `s` is already canonical: it parses and displays unchanged.
#[track_caller]
fn assert_round_trips(s: &str) {
    assert_displays_as(s, s);
}

/// Asserts that `s` is rejected. Use only where the specs give no basis for picking a variant.
#[track_caller]
fn assert_rejected(s: &str) {
    if let Ok(uri) = s.parse::<Uri>() {
        panic!("{s:?}: expected an error, got Ok({uri})");
    }
}
