use std::path::Path;

use color_eyre::Result;
use tracing_error::ErrorLayer;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use crate::config;

lazy_static::lazy_static! {
    pub static ref LOG_ENV: String = format!("{}_LOGLEVEL", config::PROJECT_NAME.clone());
    pub static ref LOG_FILE: String = format!("{}.log", env!("CARGO_PKG_NAME"));
}

pub fn init() -> Result<()> {
    let directory = config::get_data_dir();
    std::fs::create_dir_all(directory.clone())?;
    let log_path = directory.join(LOG_FILE.clone());
    rotate(&log_path, MAX_LOG_BYTES)?;
    // Append, so the log of a run that crashed is still there after the next start.
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    let env_filter = EnvFilter::builder().with_default_directive(tracing::Level::INFO.into());
    // If the `RUST_LOG` environment variable is set, use that as the default, otherwise use the
    // value of the `LOG_ENV` environment variable. If the `LOG_ENV` environment variable contains
    // errors, then this will return an error.
    let env_filter = env_filter
        .try_from_env()
        .or_else(|_| env_filter.with_env_var(LOG_ENV.clone()).from_env())?;
    let file_subscriber = fmt::layer()
        .with_file(true)
        .with_line_number(true)
        .with_writer(log_file)
        .with_target(false)
        .with_ansi(false)
        .with_filter(env_filter);
    tracing_subscriber::registry()
        .with(file_subscriber)
        .with(ErrorLayer::default())
        .try_init()?;
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        pid = std::process::id(),
        "acres-tui started"
    );
    Ok(())
}

/// The log is moved aside to `<name>.1` (replacing an older one) once it passes this size, so it
/// keeps about two runs' worth of history without growing for good.
const MAX_LOG_BYTES: u64 = 5 * 1024 * 1024;

/// Moves `path` to `path.1` if it is larger than `max` bytes.
fn rotate(path: &Path, max: u64) -> std::io::Result<()> {
    match std::fs::metadata(path) {
        Ok(meta) if meta.len() > max => {
            let mut old = path.as_os_str().to_owned();
            old.push(".1");
            std::fs::rename(path, old)
        }
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_large_log_moves_aside_and_a_small_one_stays() {
        let dir = std::env::temp_dir().join(format!("acres-tui-log-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("acres-tui.log");
        std::fs::write(&log, "0123456789").unwrap();

        rotate(&log, 100).unwrap();
        assert!(log.exists());

        rotate(&log, 5).unwrap();
        assert!(!log.exists());
        assert_eq!(
            std::fs::read_to_string(dir.join("acres-tui.log.1")).unwrap(),
            "0123456789"
        );

        rotate(&dir.join("missing.log"), 5).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
