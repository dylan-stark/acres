//! How `acres-cli` reports a failure: a stable kind, an exit code by class, and a message with a
//! hint, as text or as one line of JSON on stderr (see ADR 0019 and `docs/ERROR-HANDLING.md`).

use std::{error::Error as StdError, io, process::ExitCode};

use color_eyre::eyre::Report;
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

/// How errors are written to stderr.
#[derive(clap::ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ErrorFormat {
    /// `error: …`, then `caused by: …` and `hint: …` lines.
    #[default]
    Text,
    /// One line of JSON: `{"kind", "message", "causes", "hint", "exit_code"}`.
    Json,
}

impl ErrorFormat {
    /// The format asked for by `--error-format` (before clap has parsed anything, so usage
    /// errors can be reported in it too) or else `ACRES_ERROR_FORMAT`.
    pub fn from_args_and_env(args: &[String], env: Option<&str>) -> Self {
        let mut flag = None;
        let mut args = args.iter();
        while let Some(arg) = args.next() {
            if arg == "--error-format" {
                flag = args.next().map(String::as_str);
            } else if let Some(value) = arg.strip_prefix("--error-format=") {
                flag = Some(value);
            }
        }
        match flag.or(env) {
            Some(value) if value.eq_ignore_ascii_case("json") => ErrorFormat::Json,
            _ => ErrorFormat::Text,
        }
    }
}

/// A failure, ready to report.
#[derive(Debug, PartialEq, Serialize)]
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

/// Why a run ended without doing its job.
pub enum Outcome {
    /// Report this and exit with its code.
    Failed(Box<Failure>),
    /// stdout was closed early, as by `| head`. Not an error: exit quietly with success.
    BrokenPipe,
}

/// Classifies `report` by the first error in its chain that `acres-cli` knows.
pub fn classify(report: &Report) -> Outcome {
    let mut chain = report.chain();
    let message = chain.next().map(ToString::to_string).unwrap_or_default();
    let causes = chain.map(ToString::to_string).collect();
    let (kind, hint, exit_code) = match report.chain().find_map(known) {
        Some(Known::BrokenPipe) => return Outcome::BrokenPipe,
        Some(Known::Error { kind, hint, exit }) => (kind, hint, exit),
        None => ("other", None, exit::OTHER),
    };
    Outcome::Failed(Box::new(Failure {
        kind,
        message,
        causes,
        hint,
        exit_code,
    }))
}

/// A failure from clap's own parsing.
pub fn usage(error: &clap::Error) -> Failure {
    let rendered = error.render().to_string();
    let message = rendered
        .lines()
        .next()
        .unwrap_or_default()
        .trim_start_matches("error: ")
        .to_string();
    Failure {
        kind: "usage",
        message,
        causes: Vec::new(),
        hint: Some("run with --help to see the commands and options"),
        exit_code: exit::USAGE,
    }
}

/// Writes `failure` to stderr in `format` and returns its exit code.
pub fn emit(failure: &Failure, format: ErrorFormat) -> ExitCode {
    let text = match format {
        ErrorFormat::Text => text(failure),
        ErrorFormat::Json => {
            serde_json::to_string(failure).expect("a failure is plain strings and numbers") + "\n"
        }
    };
    let _ = io::Write::write_all(&mut io::stderr(), text.as_bytes());
    ExitCode::from(failure.exit_code)
}

fn text(failure: &Failure) -> String {
    let mut text = format!("error: {}\n", failure.message);
    for cause in &failure.causes {
        text.push_str(&format!("  caused by: {cause}\n"));
    }
    if let Some(hint) = failure.hint {
        text.push_str(&format!("hint: {hint}\n"));
    }
    text
}

enum Known {
    BrokenPipe,
    Error {
        kind: &'static str,
        hint: Option<&'static str>,
        exit: u8,
    },
}

fn known(error: &(dyn StdError + 'static)) -> Option<Known> {
    if let Some(e) = error.downcast_ref::<acres::Error>() {
        return Some(acres_error(e));
    }
    if let Some(e) = error.downcast_ref::<acres::ResponseError>() {
        return Some(response(e));
    }
    if let Some(e) = error.downcast_ref::<acres::RequestError>() {
        return Some(invalid(e.kind(), e.hint()));
    }
    if let Some(e) = error.downcast_ref::<acres::CacheError>() {
        return Some(cache(e));
    }
    if let Some(e) = error.downcast_ref::<acres::ParseError>() {
        return Some(parse(e));
    }
    if let Some(e) = error.downcast_ref::<iiif::Error>() {
        return Some(invalid(e.kind(), None));
    }
    if let Some(e) = error.downcast_ref::<iiif::BaseUriError>() {
        return Some(invalid(e.kind(), None));
    }
    if let Some(e) = error.downcast_ref::<iiif::ParameterError>() {
        return Some(invalid(e.kind(), None));
    }
    if let Some(e) = error.downcast_ref::<iiif::RequestError>() {
        return Some(invalid(e.kind(), None));
    }
    if let Some(e) = error.downcast_ref::<charaoke::Error>() {
        return Some(invalid(e.kind(), None));
    }
    if error.downcast_ref::<image::ImageError>().is_some() {
        return Some(invalid(
            "input.image",
            Some(
                "the input must be a JPEG or PNG image, such as `acres-cli iiif --to bytes` writes",
            ),
        ));
    }
    if error.downcast_ref::<clap_stdin::StdinError>().is_some() {
        return Some(invalid(
            "input.read",
            Some("check the input path, or pass - to read stdin"),
        ));
    }
    if let Some(e) = error.downcast_ref::<io::Error>() {
        if e.kind() == io::ErrorKind::BrokenPipe {
            return Some(Known::BrokenPipe);
        }
        return Some(Known::Error {
            kind: "io",
            hint: None,
            exit: exit::ENVIRONMENT,
        });
    }
    None
}

fn invalid(kind: &'static str, hint: Option<&'static str>) -> Known {
    Known::Error {
        kind,
        hint,
        exit: exit::INVALID_INPUT,
    }
}

fn acres_error(e: &acres::Error) -> Known {
    match e {
        acres::Error::Response(e) => response(e),
        acres::Error::Cache(e) => cache(e),
        acres::Error::Parse(e) => parse(e),
        acres::Error::Config(_) => Known::Error {
            kind: e.kind(),
            hint: e.hint(),
            exit: exit::ENVIRONMENT,
        },
        _ => invalid(e.kind(), e.hint()),
    }
}

fn response(e: &acres::ResponseError) -> Known {
    let exit = if e.kind() == "response.not_found" {
        exit::NOT_FOUND
    } else if e.is_transient() {
        exit::TRANSIENT
    } else {
        exit::INVALID_INPUT
    };
    Known::Error {
        kind: e.kind(),
        hint: e.hint(),
        exit,
    }
}

fn cache(e: &acres::CacheError) -> Known {
    let exit = match e {
        acres::CacheError::Missing { .. } => exit::OFFLINE_MISS,
        _ => exit::ENVIRONMENT,
    };
    Known::Error {
        kind: e.kind(),
        hint: e.hint(),
        exit,
    }
}

fn parse(e: &acres::ParseError) -> Known {
    let exit = match e {
        acres::ParseError::InvalidBody { input, .. } if !input.starts_with("http") => {
            exit::INVALID_INPUT
        }
        _ => exit::BAD_RESPONSE,
    };
    Known::Error {
        kind: e.kind(),
        hint: e.hint(),
        exit,
    }
}

#[cfg(test)]
mod tests {
    use color_eyre::eyre::{WrapErr, eyre};

    use super::*;

    fn failure(report: Report) -> Failure {
        match classify(&report) {
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
    fn a_broken_pipe_is_not_a_failure() {
        let error = io::Error::from(io::ErrorKind::BrokenPipe);
        let report = Err::<(), _>(error).wrap_err("failed to write").unwrap_err();
        assert!(matches!(classify(&report), Outcome::BrokenPipe));
    }

    #[test]
    fn unknown_errors_are_other() {
        let failure = failure(eyre!("something odd"));
        assert_eq!(failure.kind, "other");
        assert_eq!(failure.exit_code, exit::OTHER);
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
        let json = serde_json::to_string(&failure).unwrap();
        assert_eq!(
            json,
            r#"{"kind":"cache.missing","message":"not cached","causes":[],"hint":null,"exit_code":4}"#
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
            text(&failure),
            "error: could not fetch\n  caused by: not cached\nhint: go online\n"
        );
    }

    #[test]
    fn the_format_comes_from_the_flag_then_the_environment() {
        let args = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            ErrorFormat::from_args_and_env(&args(&["acres-cli", "--error-format", "json"]), None),
            ErrorFormat::Json
        );
        assert_eq!(
            ErrorFormat::from_args_and_env(&args(&["acres-cli", "--error-format=json"]), None),
            ErrorFormat::Json
        );
        assert_eq!(
            ErrorFormat::from_args_and_env(&args(&["acres-cli"]), Some("json")),
            ErrorFormat::Json
        );
        assert_eq!(
            ErrorFormat::from_args_and_env(
                &args(&["acres-cli", "--error-format", "text"]),
                Some("json")
            ),
            ErrorFormat::Text
        );
    }
}
