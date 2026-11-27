//! Logs to stderr, at a level set by `-v`/`-q` or by `RUST_LOG` (see ADR 0020).

use std::io::IsTerminal;

use color_eyre::Result;
use tracing::level_filters::LevelFilter;
use tracing_error::ErrorLayer;
use tracing_subscriber::{EnvFilter, fmt, fmt::format::FmtSpan, prelude::*};

/// How much to log, from the command-line flags.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verbosity {
    /// `-q`: nothing. Errors are still reported, since they don't go through the log.
    Quiet,
    /// The default: warnings, such as a shared rate limit that can't be used.
    Normal,
    /// `-v`, `-vv` or `-vvv`.
    Verbose(u8),
}

impl Verbosity {
    /// The verbosity for `-v` given `count` times, or `-q`.
    pub fn from_flags(count: u8, quiet: bool) -> Self {
        match (quiet, count) {
            (true, _) => Verbosity::Quiet,
            (false, 0) => Verbosity::Normal,
            (false, n) => Verbosity::Verbose(n),
        }
    }

    fn level(self) -> LevelFilter {
        match self {
            Verbosity::Quiet => LevelFilter::OFF,
            Verbosity::Normal => LevelFilter::WARN,
            Verbosity::Verbose(1) => LevelFilter::INFO,
            Verbosity::Verbose(2) => LevelFilter::DEBUG,
            Verbosity::Verbose(_) => LevelFilter::TRACE,
        }
    }
}

/// Installs the stderr logger. `RUST_LOG`, when set, wins over `verbosity`.
pub fn init(verbosity: Verbosity) -> Result<()> {
    let filter = match std::env::var("RUST_LOG") {
        Ok(directives) if !directives.is_empty() => EnvFilter::try_new(directives)?,
        _ => EnvFilter::try_new(filter_directive(verbosity))?,
    };
    let stderr = std::io::stderr();
    let ansi = stderr.is_terminal() && std::env::var_os("NO_COLOR").is_none();
    let spans = span_events(&filter);
    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_writer(std::io::stderr)
                .with_ansi(ansi)
                .with_target(false)
                .with_span_events(spans)
                .with_filter(filter),
        )
        .with(ErrorLayer::default())
        .try_init()?;
    Ok(())
}

/// At TRACE, each span also logs a `close` line with its `time.busy` and `time.idle`, so `-vvv`
/// shows how long every request took end to end.
pub fn span_events(filter: &EnvFilter) -> FmtSpan {
    match filter.max_level_hint() {
        Some(level) if level < LevelFilter::TRACE => FmtSpan::NONE,
        _ => FmtSpan::CLOSE,
    }
}

/// INFO everywhere, plus the DEBUG `fetched` / `request failed` event that ends each request,
/// without the cache and rate-limit detail in the modules under `acres::catalog`.
const REQUESTS: &str = "info,acres::catalog=debug,acres::catalog::cache=info,\
                        acres::catalog::limiter=info,acres::catalog::shared=info";

/// `-v` shows each request's outcome, which is a DEBUG event in `acres`, without the DEBUG detail
/// of everything else. `-vv` and up apply to every crate.
fn filter_directive(verbosity: Verbosity) -> String {
    match verbosity {
        Verbosity::Verbose(1) => REQUESTS.to_string(),
        other => other.level().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spans_log_their_close_only_at_trace() {
        let filter = |d: &str| EnvFilter::try_new(d).unwrap();
        assert_eq!(span_events(&filter("debug")), FmtSpan::NONE);
        assert_eq!(span_events(&filter("trace")), FmtSpan::CLOSE);
        assert_eq!(span_events(&filter("info,acres=trace")), FmtSpan::CLOSE);
    }

    #[test]
    fn flags_pick_the_level() {
        assert_eq!(filter_directive(Verbosity::from_flags(0, true)), "off");
        assert_eq!(filter_directive(Verbosity::from_flags(0, false)), "warn");
        assert_eq!(filter_directive(Verbosity::from_flags(1, false)), REQUESTS);
        for directive in REQUESTS.split(',') {
            directive
                .parse::<tracing_subscriber::filter::Directive>()
                .unwrap();
        }
        assert_eq!(filter_directive(Verbosity::from_flags(2, false)), "debug");
        assert_eq!(filter_directive(Verbosity::from_flags(5, false)), "trace");
    }
}
