//! How the TUI reports a failure: classified as the CLI does, with a kind and a hint (see
//! `acres-report`), and logged once with its `kind`.

use std::error::Error as StdError;

pub use acres_report::Failure;
use acres_report::{Class, Outcome, exit};

/// Classifies `error` and logs it once at WARN, with its `kind`.
///
/// For an `eyre::Report`, pass `report.as_ref()`, so the context added with `wrap_err` stays
/// in the message.
pub fn failed(error: &(dyn StdError + 'static)) -> Failure {
    let failure = Failure::from_error(error);
    tracing::warn!(kind = failure.kind, error = %failure.summary(), "failed");
    failure
}

/// The failure and its hint on one line, for the status bar and the panes.
pub fn line(failure: &Failure) -> String {
    match failure.hint {
        Some(hint) => format!("{} · {hint}", failure.summary()),
        None => failure.summary(),
    }
}

/// Classifies a failure that stops the TUI before or while it runs, to report on stderr in the
/// CLI's format (`error:`, `caused by:`, `hint:`) and exit with its code.
pub fn at_exit(error: &(dyn StdError + 'static)) -> Failure {
    let outcome = acres_report::classify_with(error, |error| {
        if error.downcast_ref::<config::ConfigError>().is_some() {
            return Some(Class {
                kind: "config.invalid",
                hint: Some(
                    "fix it in the config file (acres-tui --version prints where that is), or remove it",
                ),
                exit_code: exit::ENVIRONMENT,
            });
        }
        error
            .downcast_ref::<clap_stdin::StdinError>()
            .map(|_| Class {
                kind: "input.read",
                hint: Some("check the --from-file path, or pass - to read stdin"),
                exit_code: exit::INVALID_INPUT,
            })
    });
    match outcome {
        Outcome::Failed(failure) => *failure,
        Outcome::BrokenPipe => Failure::from_error(error),
    }
}

#[cfg(test)]
mod tests {
    use color_eyre::eyre::{self, WrapErr};

    use super::*;

    #[test]
    fn a_bad_config_file_is_an_environment_failure() {
        let error = Err::<(), _>(config::ConfigError::Message(
            "unknown key `<Ctrl-Q>`".into(),
        ))
        .wrap_err("could not load the config")
        .unwrap_err();
        let failure = at_exit(error.as_ref());
        assert_eq!(failure.kind, "config.invalid");
        assert_eq!(failure.exit_code, exit::ENVIRONMENT);
        assert_eq!(
            failure.to_text().lines().next(),
            Some("error: could not load the config")
        );
    }

    #[test]
    fn a_bad_acres_setting_keeps_its_own_kind_and_hint() {
        let report = eyre::Report::new(acres::Error::from(acres::ConfigError::Invalid(
            "ACRES_OFFLINE: not a bool".into(),
        )));
        let failure = at_exit(report.as_ref());
        assert_eq!(failure.kind, "config.invalid");
        assert_eq!(failure.exit_code, exit::ENVIRONMENT);
    }

    #[test]
    fn the_line_carries_the_hint() {
        let failure = Failure::from_error(&acres::CacheError::Missing {
            endpoint: "e".into(),
        });
        assert!(line(&failure).starts_with("not cached while offline: e · go online"));
    }
}
