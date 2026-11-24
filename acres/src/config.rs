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
        let builder = config::Config::builder()
            .set_default("use_cache", true)?
            .set_default("cache_dir", cache_dir.to_str().expect("path is valid"))?
            .set_default("base_uri", DEFAULT_BASE_URI.to_string())?
            .set_default("offline", false)?
            .add_source(config::Environment::with_prefix("ACRES"));
        let cfg: Self = builder.build()?.try_deserialize()?;
        tracing::debug!(?cfg);
        Ok(cfg)
    }
}

pub(crate) fn get_acres_cache_dir() -> PathBuf {
    if let Some(proj_dirs) = project_directory() {
        proj_dirs.data_local_dir().to_path_buf()
    } else {
        PathBuf::from(".").join(".cache")
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
