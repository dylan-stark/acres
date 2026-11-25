use std::{env, path::PathBuf};

use directories::ProjectDirs;
use serde::Deserialize;

/// The public AIC API.
pub const DEFAULT_BASE_URI: &str = "https://api.artic.edu/api/v1";

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub use_cache: bool,
    pub cache_dir: PathBuf,
    /// Where state shared between runs lives, such as the rate limit (`ACRES_STATE_DIR`).
    pub state_dir: PathBuf,
    pub base_uri: String,
    /// Answer only from the cache (`ACRES_OFFLINE`).
    #[serde(default)]
    pub offline: bool,
    /// Who to contact about these requests, added to `AIC-User-Agent` (`ACRES_CONTACT`).
    #[serde(default)]
    pub contact: Option<String>,
}

impl Config {
    pub fn new() -> Result<Self, config::ConfigError> {
        let cache_dir = get_acres_cache_dir();
        let state_dir = get_acres_state_dir();
        let builder = config::Config::builder()
            .set_default("use_cache", true)?
            .set_default("cache_dir", cache_dir.to_str().expect("path is valid"))?
            .set_default("state_dir", state_dir.to_str().expect("path is valid"))?
            .set_default("base_uri", DEFAULT_BASE_URI.to_string())?
            .set_default("offline", false)?
            .add_source(config::Environment::with_prefix("ACRES"));
        let cfg: Self = builder.build()?.try_deserialize()?;
        tracing::debug!(?cfg);
        Ok(cfg)
    }
}

/// The app's cache directory: `$XDG_CACHE_HOME/acres` on Linux, `~/Library/Caches` on macOS
/// and the local app data `cache` folder on Windows. Everything in it can be fetched again.
pub(crate) fn get_acres_cache_dir() -> PathBuf {
    if let Some(proj_dirs) = project_directory() {
        proj_dirs.cache_dir().to_path_buf()
    } else {
        PathBuf::from(".").join(".cache")
    }
}

/// The app's state directory: `$XDG_STATE_HOME/acres` on Linux. macOS and Windows have no
/// separate state directory, so it is the local data directory there.
pub(crate) fn get_acres_state_dir() -> PathBuf {
    match project_directory() {
        Some(dirs) => dirs
            .state_dir()
            .unwrap_or_else(|| dirs.data_local_dir())
            .to_path_buf(),
        None => PathBuf::from(".").join(".state"),
    }
}

fn project_directory() -> Option<ProjectDirs> {
    ProjectDirs::from("com", "acres", env!("CARGO_PKG_NAME"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_using_cache() {
        assert!(Config::new().unwrap().use_cache);
    }
}
