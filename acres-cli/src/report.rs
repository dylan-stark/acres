//! How `acres-cli` reports a failure: as text or as one line of JSON on stderr, with the kind,
//! hint and exit code from `acres-report` (see ADRs 0019 and 0021, and `docs/ERROR-HANDLING.md`).

use std::{io, process::ExitCode};

use acres_report::Class;
use color_eyre::eyre::Report;

pub use acres_report::{Failure, Outcome, exit};

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

/// Classifies `report` by the first error in its chain that `acres-cli` knows.
pub fn classify(report: &Report) -> Outcome {
    acres_report::classify_with(report.as_ref(), |error| {
        // `clap_stdin` wraps the io error without exposing it as a source.
        error
            .downcast_ref::<clap_stdin::StdinError>()
            .map(|_| Class {
                kind: "input.read",
                hint: Some("check the input path, or pass - to read stdin"),
                exit_code: exit::INVALID_INPUT,
            })
    })
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
        ErrorFormat::Text => failure.to_text(),
        ErrorFormat::Json => failure.to_json(),
    };
    let _ = io::Write::write_all(&mut io::stderr(), text.as_bytes());
    ExitCode::from(failure.exit_code)
}

#[cfg(test)]
mod tests {
    use super::*;

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
