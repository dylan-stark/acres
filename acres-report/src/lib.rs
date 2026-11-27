#![deny(missing_docs)]

//! How the acres apps classify and report a failure: a stable kind, a hint on what to do next,
//! and an exit code by class (see ADRs 0019 and 0021, and `docs/ERROR-HANDLING.md`).
//!
//! The libraries know their own errors' kinds and hints. Which exit code a class of failure
//! gets, and how a failure is written out, is the apps' policy, so it lives here, shared by
//! `acres-cli` and `acres-tui`.

use std::{error::Error as StdError, io};

use serde::Serialize;

/// The exit codes, by class of failure. They are part of the CLI's contract: a code is never
/// reused for another class.
pub mod exit {
    /// Something not covered below. Includes bugs.
    pub const OTHER: u8 = 1;
    /// The command line could not be parsed (clap's own code).
    pub const USAGE: u8 = 2;
    /// The API has no such record (404).
    pub const NOT_FOUND: u8 = 3;
    /// Offline, and the cache has no entry for the request.
    pub const OFFLINE_MISS: u8 = 4;
    /// Rate limited, network trouble or an API server error: trying again later may work.
    pub const TRANSIENT: u8 = 5;
    /// The request or its input is invalid, such as a malformed IIIF region or a page past the
    /// first 1,000 results.
    pub const INVALID_INPUT: u8 = 6;
    /// The local environment failed: configuration, the cache directory or reading and writing
    /// files.
    pub const ENVIRONMENT: u8 = 7;
    /// The API answered with something acres can't read.
    pub const BAD_RESPONSE: u8 = 8;
}

/// A failure, ready to report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Failure {
    /// A stable, dotted name, such as `cache.missing` (see [`acres::Error::kind`]).
    pub kind: &'static str,
    /// The outermost message.
    pub message: String,
    /// The causes under it, outermost first.
    pub causes: Vec<String>,
    /// What to do next, when there is something to say.
    pub hint: Option<&'static str>,
    /// The process exit code (see [`exit`]).
    pub exit_code: u8,
}

impl Failure {
    /// Classifies `error` and its chain of sources. A broken pipe counts as an `io` failure
    /// here: use [`classify`] where it should end the run quietly instead.
    pub fn from_error(error: &(dyn StdError + 'static)) -> Self {
        failure(error, known(error))
    }

    /// The message and its causes on one line, as `message: cause: cause`.
    pub fn summary(&self) -> String {
        let mut summary = self.message.clone();
        for cause in &self.causes {
            summary.push_str(": ");
            summary.push_str(cause);
        }
        summary
    }

    /// The failure as text: `error: …`, then a `caused by: …` line for each cause and a
    /// `hint: …` line.
    pub fn to_text(&self) -> String {
        let mut text = format!("error: {}\n", self.message);
        for cause in &self.causes {
            text.push_str(&format!("  caused by: {cause}\n"));
        }
        if let Some(hint) = self.hint {
            text.push_str(&format!("hint: {hint}\n"));
        }
        text
    }

    /// The failure as one line of JSON, ending in a newline:
    /// `{"kind", "message", "causes", "hint", "exit_code"}`.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("a failure is plain strings and numbers") + "\n"
    }
}

/// What an app knows about one error in a chain: its kind, hint and exit code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Class {
    /// See [`Failure::kind`].
    pub kind: &'static str,
    /// See [`Failure::hint`].
    pub hint: Option<&'static str>,
    /// See [`Failure::exit_code`].
    pub exit_code: u8,
}

/// Why a run ended without doing its job.
#[derive(Debug)]
pub enum Outcome {
    /// Report this and exit with its code.
    Failed(Box<Failure>),
    /// stdout was closed early, as by `| head`. Not an error: exit quietly with success.
    BrokenPipe,
}

/// Classifies `error` by the first error in its chain that is known here.
///
/// For an `eyre::Report`, pass `report.as_ref()`: its chain then includes the context added
/// with `wrap_err`.
pub fn classify(error: &(dyn StdError + 'static)) -> Outcome {
    classify_with(error, |_| None)
}

/// The same, trying `extra` on each error in the chain first, for errors only the app knows.
pub fn classify_with(
    error: &(dyn StdError + 'static),
    extra: impl Fn(&(dyn StdError + 'static)) -> Option<Class>,
) -> Outcome {
    let found = chain(error).find_map(|e| extra(e).map(Known::Class).or_else(|| known_one(e)));
    match found {
        Some(Known::BrokenPipe) => Outcome::BrokenPipe,
        found => Outcome::Failed(Box::new(failure(error, found))),
    }
}

fn chain<'a>(
    error: &'a (dyn StdError + 'static),
) -> impl Iterator<Item = &'a (dyn StdError + 'static)> {
    std::iter::successors(Some(error), |&e| e.source())
}

fn failure(error: &(dyn StdError + 'static), found: Option<Known>) -> Failure {
    let mut messages = chain(error).map(ToString::to_string);
    let message = messages.next().unwrap_or_default();
    let causes = messages.collect();
    let Class {
        kind,
        hint,
        exit_code,
    } = match found {
        Some(Known::Class(class)) => class,
        Some(Known::BrokenPipe) => class("io", None, exit::ENVIRONMENT),
        None => class("other", None, exit::OTHER),
    };
    Failure {
        kind,
        message,
        causes,
        hint,
        exit_code,
    }
}

enum Known {
    BrokenPipe,
    Class(Class),
}

fn known(error: &(dyn StdError + 'static)) -> Option<Known> {
    chain(error).find_map(known_one)
}

fn class(kind: &'static str, hint: Option<&'static str>, exit_code: u8) -> Class {
    Class {
        kind,
        hint,
        exit_code,
    }
}

fn known_one(error: &(dyn StdError + 'static)) -> Option<Known> {
    if let Some(e) = error.downcast_ref::<acres::Error>() {
        return Some(Known::Class(acres_error(e)));
    }
    if let Some(e) = error.downcast_ref::<acres::ResponseError>() {
        return Some(Known::Class(response(e)));
    }
    if let Some(e) = error.downcast_ref::<acres::RequestError>() {
        return Some(Known::Class(invalid(e.kind(), e.hint())));
    }
    if let Some(e) = error.downcast_ref::<acres::CacheError>() {
        return Some(Known::Class(cache(e)));
    }
    if let Some(e) = error.downcast_ref::<acres::ParseError>() {
        return Some(Known::Class(parse(e)));
    }
    if let Some(e) = error.downcast_ref::<acres::ConfigError>() {
        return Some(Known::Class(class(e.kind(), e.hint(), exit::ENVIRONMENT)));
    }
    if let Some(e) = error.downcast_ref::<iiif::Error>() {
        return Some(Known::Class(invalid(e.kind(), None)));
    }
    if let Some(e) = error.downcast_ref::<iiif::BaseUriError>() {
        return Some(Known::Class(invalid(e.kind(), None)));
    }
    if let Some(e) = error.downcast_ref::<iiif::ParameterError>() {
        return Some(Known::Class(invalid(e.kind(), None)));
    }
    if let Some(e) = error.downcast_ref::<iiif::RequestError>() {
        return Some(Known::Class(invalid(e.kind(), None)));
    }
    if let Some(e) = error.downcast_ref::<charaoke::Error>() {
        return Some(Known::Class(invalid(e.kind(), None)));
    }
    if error.downcast_ref::<image::ImageError>().is_some() {
        return Some(Known::Class(invalid(
            "input.image",
            Some(
                "the input must be a JPEG or PNG image, such as `acres-cli iiif --to bytes` writes",
            ),
        )));
    }
    if let Some(e) = error.downcast_ref::<io::Error>() {
        if e.kind() == io::ErrorKind::BrokenPipe {
            return Some(Known::BrokenPipe);
        }
        return Some(Known::Class(class("io", None, exit::ENVIRONMENT)));
    }
    None
}

fn invalid(kind: &'static str, hint: Option<&'static str>) -> Class {
    class(kind, hint, exit::INVALID_INPUT)
}

fn acres_error(e: &acres::Error) -> Class {
    match e {
        acres::Error::Response(e) => response(e),
        acres::Error::Cache(e) => cache(e),
        acres::Error::Parse(e) => parse(e),
        acres::Error::Config(_) => class(e.kind(), e.hint(), exit::ENVIRONMENT),
        _ => invalid(e.kind(), e.hint()),
    }
}

fn response(e: &acres::ResponseError) -> Class {
    let exit = if e.kind() == "response.not_found" {
        exit::NOT_FOUND
    } else if e.is_transient() {
        exit::TRANSIENT
    } else {
        exit::INVALID_INPUT
    };
    class(e.kind(), e.hint(), exit)
}

fn cache(e: &acres::CacheError) -> Class {
    let exit = match e {
        acres::CacheError::Missing { .. } => exit::OFFLINE_MISS,
        _ => exit::ENVIRONMENT,
    };
    class(e.kind(), e.hint(), exit)
}

fn parse(e: &acres::ParseError) -> Class {
    let exit = match e {
        acres::ParseError::InvalidBody { input, .. } if !input.starts_with("http") => {
            exit::INVALID_INPUT
        }
        _ => exit::BAD_RESPONSE,
    };
    class(e.kind(), e.hint(), exit)
}

#[cfg(test)]
mod tests {
    use eyre::{Report, WrapErr, eyre};

    use super::*;

    fn failure(report: Report) -> Failure {
        match classify(report.as_ref()) {
            Outcome::Failed(failure) => *failure,
            Outcome::BrokenPipe => panic!("not a broken pipe"),
        }
    }

    #[test]
    fn an_offline_miss_has_its_own_kind_code_and_hint() {
        let error = acres::Error::from(acres::CacheError::Missing {
            endpoint: "https://api.artic.edu/api/v1/artworks/1".into(),
        });
        let failure = failure(error.into());
        assert_eq!(failure.kind, "cache.missing");
        assert_eq!(failure.exit_code, exit::OFFLINE_MISS);
        assert!(failure.hint.unwrap().contains("offline"));
        assert_eq!(
            failure.message,
            "not cached while offline: https://api.artic.edu/api/v1/artworks/1"
        );
    }

    #[test]
    fn context_stays_in_the_message_and_the_kind_comes_from_the_cause() {
        let error = acres::Error::from(acres::ResponseError::Status {
            endpoint: "e".into(),
            status: 404,
            error: "Not found".into(),
            detail: String::new(),
        });
        let report = Err::<(), _>(error)
            .wrap_err("could not fetch the artwork")
            .unwrap_err();
        let failure = failure(report);
        assert_eq!(failure.kind, "response.not_found");
        assert_eq!(failure.exit_code, exit::NOT_FOUND);
        assert_eq!(failure.message, "could not fetch the artwork");
        assert_eq!(failure.causes, ["404 from e: Not found: "]);
        assert_eq!(
            failure.summary(),
            "could not fetch the artwork: 404 from e: Not found: "
        );
    }

    #[test]
    fn transient_responses_say_try_again() {
        let error = acres::Error::from(acres::ResponseError::TooManyRequests {
            endpoint: "e".into(),
        });
        assert_eq!(failure(error.into()).exit_code, exit::TRANSIENT);
    }

    #[test]
    fn iiif_errors_are_invalid_input() {
        let error = "nope".parse::<iiif::Region>().unwrap_err();
        let failure = failure(error.into());
        assert_eq!(failure.kind, "iiif.parameter.invalid_region");
        assert_eq!(failure.exit_code, exit::INVALID_INPUT);
    }

    #[test]
    fn a_broken_pipe_is_not_a_failure_unless_asked_for_one() {
        let error = io::Error::from(io::ErrorKind::BrokenPipe);
        let report = Err::<(), _>(error).wrap_err("failed to write").unwrap_err();
        assert!(matches!(classify(report.as_ref()), Outcome::BrokenPipe));
        let failure = Failure::from_error(report.as_ref());
        assert_eq!((failure.kind, failure.exit_code), ("io", exit::ENVIRONMENT));
    }

    #[test]
    fn unknown_errors_are_other() {
        let failure = failure(eyre!("something odd"));
        assert_eq!(failure.kind, "other");
        assert_eq!(failure.exit_code, exit::OTHER);
    }

    #[test]
    fn extra_classes_come_first() {
        let report = eyre!("something odd");
        let outcome = classify_with(report.as_ref(), |_| {
            Some(Class {
                kind: "input.read",
                hint: None,
                exit_code: exit::INVALID_INPUT,
            })
        });
        let Outcome::Failed(failure) = outcome else {
            panic!("not a broken pipe");
        };
        assert_eq!(failure.kind, "input.read");
    }

    #[test]
    fn json_is_one_line_with_every_field() {
        let failure = Failure {
            kind: "cache.missing",
            message: "not cached".into(),
            causes: vec![],
            hint: None,
            exit_code: 4,
        };
        assert_eq!(
            failure.to_json(),
            "{\"kind\":\"cache.missing\",\"message\":\"not cached\",\"causes\":[],\"hint\":null,\"exit_code\":4}\n"
        );
    }

    #[test]
    fn text_puts_causes_and_hint_on_their_own_lines() {
        let failure = Failure {
            kind: "cache.missing",
            message: "could not fetch".into(),
            causes: vec!["not cached".into()],
            hint: Some("go online"),
            exit_code: 4,
        };
        assert_eq!(
            failure.to_text(),
            "error: could not fetch\n  caused by: not cached\nhint: go online\n"
        );
    }
}
