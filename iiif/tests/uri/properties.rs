//! Property tests for parsing and displaying base URIs.
//!
//! The reference encoder below spells out the canonical form these tests expect: IIIF §9's
//! to-encode set, plus everything RFC 3986 doesn't allow raw in a path segment, plus every
//! non-ASCII byte, with uppercase hex (RFC 3986 §2.1). Only ALPHA, DIGIT, unreserved
//! `-._~`, sub-delims `!$&'()*+,;=` and `:` stay raw.

use iiif::Uri;
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use proptest::prelude::*;

const CANONICAL: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~')
    .remove(b'!')
    .remove(b'$')
    .remove(b'&')
    .remove(b'\'')
    .remove(b'(')
    .remove(b')')
    .remove(b'*')
    .remove(b'+')
    .remove(b',')
    .remove(b';')
    .remove(b'=')
    .remove(b':');

fn encode(s: &str) -> String {
    utf8_percent_encode(s, CANONICAL).to_string()
}

/// Bytes RFC 3986 allows raw in a canonical IIIF path: unreserved, sub-delims, ":", the "/"
/// separator, and "%" starting an escape.
fn is_canonical_path_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b"-._~!$&'()*+,;=:/%".contains(&b)
}

/// Any string that names an addressable path segment: non-empty and not a dot segment
/// (RFC 3986 §5.2.4).
fn segment() -> impl Strategy<Value = String> {
    any::<String>().prop_filter("addressable segment", |s| {
        !s.is_empty() && s != "." && s != ".."
    })
}

/// A base URI in canonical form, built from arbitrary components.
fn canonical_uri() -> impl Strategy<Value = String> {
    let scheme = prop_oneof!["http", "https"];
    let host = "[a-z]{1,10}(\\.[a-z]{1,10}){0,2}";
    let port = proptest::option::of(1024_u16..);
    let prefix = proptest::collection::vec(segment(), 0..4);
    (scheme, host, port, prefix, segment()).prop_map(|(scheme, host, port, prefix, id)| {
        let port = port.map(|p| format!(":{p}")).unwrap_or_default();
        let prefix: String = prefix.iter().map(|s| format!("/{}", encode(s))).collect();
        format!("{scheme}://{host}{port}{prefix}/{}", encode(&id))
    })
}

/// Strings that look roughly like base URIs, valid or not, to exercise every parsing branch.
fn uri_like() -> impl Strategy<Value = String> {
    prop_oneof![
        "(https?|HTTPS?|ftp|file)://[a-zA-Z0-9.:@\\[\\]-]{0,12}(/[ -~]{0,8}){0,4}",
        "(https?|HTTPS?|ftp|file)://[a-zA-Z0-9.:@\\[\\]-]{0,12}(/(%[0-9a-fA-F]{0,2}|[a-z.]|\\PC){0,6}){0,4}",
        "https?:[ -~]{0,24}",
    ]
}

/// The path of a displayed URI: everything after the authority.
fn path_of(displayed: &str) -> &str {
    let after_scheme = displayed
        .split_once("://")
        .map_or(displayed, |(_, rest)| rest);
    after_scheme.find('/').map_or("", |i| &after_scheme[i..])
}

proptest! {
    /// IIIF §9: a correctly encoded base URI parses and displays unchanged.
    #[test]
    fn canonical_uris_round_trip(s in canonical_uri()) {
        let uri: Uri = s.parse().map_err(|e| TestCaseError::fail(format!("{e:?}")))?;
        prop_assert_eq!(uri.to_string(), s);
    }

    /// IIIF §3: the identifier is whatever the client encoded. Any identifier, encoded and
    /// placed after any prefix, is recovered intact: re-encoding it gives the same segment.
    #[test]
    fn identifier_is_recovered_exactly(prefix in proptest::collection::vec(segment(), 0..3), id in segment()) {
        let prefix: String = prefix.iter().map(|s| format!("/{}", encode(s))).collect();
        let s = format!("https://example.org{prefix}/{}", encode(&id));
        let uri: Uri = s.parse().map_err(|e| TestCaseError::fail(format!("{e:?}")))?;
        let displayed = uri.to_string();
        let (_, last) = displayed.rsplit_once('/').expect("displayed URI has a path");
        prop_assert_eq!(last, encode(&id));
    }

    /// RFC 3986 §6.2.2: percent-encoding every identifier byte, with uppercase or lowercase
    /// hex, gives an equal URI.
    #[test]
    fn identifier_spellings_are_equivalent(id in segment()) {
        let canonical: Uri = format!("https://example.org/{}", encode(&id))
            .parse()
            .map_err(|e| TestCaseError::fail(format!("{e:?}")))?;

        let upper: String = id.bytes().map(|b| format!("%{b:02X}")).collect();
        let lower: String = id.bytes().map(|b| format!("%{b:02x}")).collect();
        for spelling in [upper, lower] {
            let variant: Uri = format!("https://example.org/{spelling}")
                .parse()
                .map_err(|e| TestCaseError::fail(format!("{spelling}: {e:?}")))?;
            prop_assert_eq!(&variant, &canonical);
        }
    }

    /// Parsing never panics, whatever the input.
    #[test]
    fn parsing_arbitrary_strings_never_panics(s in any::<String>()) {
        let _ = s.parse::<Uri>();
    }

    /// Parsing never panics on input shaped like a URI.
    #[test]
    fn parsing_uri_like_strings_never_panics(s in uri_like()) {
        let _ = s.parse::<Uri>();
    }

    /// Display is a fixed point: once parsed, displaying and re-parsing changes nothing.
    #[test]
    fn display_is_idempotent(s in prop_oneof![canonical_uri(), uri_like()]) {
        if let Ok(uri) = s.parse::<Uri>() {
            let displayed = uri.to_string();
            let reparsed: Uri = displayed
                .parse()
                .map_err(|e| TestCaseError::fail(format!("{displayed}: {e:?}")))?;
            prop_assert_eq!(&reparsed, &uri);
            prop_assert_eq!(reparsed.to_string(), displayed);
        }
    }

    /// RFC 3986 §2, IIIF §9: whatever was accepted displays as a valid canonical path. Only
    /// permitted characters appear raw, and every escape is `%` plus two uppercase hex digits.
    #[test]
    fn display_is_canonical(s in prop_oneof![canonical_uri(), uri_like()]) {
        if let Ok(uri) = s.parse::<Uri>() {
            let displayed = uri.to_string();
            let path = path_of(&displayed).as_bytes();
            prop_assert!(
                path.iter().all(|&b| is_canonical_path_byte(b)),
                "non-canonical byte in {displayed:?}"
            );
            for (i, _) in path.iter().enumerate().filter(|&(_, &b)| b == b'%') {
                let escape = path.get(i + 1..i + 3);
                prop_assert!(
                    matches!(escape, Some([h, l]) if [h, l].iter().all(|c| c.is_ascii_digit() || (b'A'..=b'F').contains(c))),
                    "bad escape at {i} in {displayed:?}"
                );
            }
        }
    }

    /// Serialization keeps the full value.
    #[test]
    fn serde_round_trips(s in canonical_uri()) {
        let uri: Uri = s.parse().map_err(|e| TestCaseError::fail(format!("{e:?}")))?;
        let json = serde_json::to_string(&uri).expect("serializes");
        let back: Uri = serde_json::from_str(&json).expect("deserializes");
        prop_assert_eq!(back, uri);
    }
}
