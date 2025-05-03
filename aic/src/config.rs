use std::{env, path::PathBuf};

use directories::ProjectDirs;
use lazy_static::lazy_static;
use serde::Deserialize;

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub aic_cache_dir: PathBuf,
}

lazy_static! {
    pub static ref AIC_CACHE_DIR: Option<PathBuf> =
        env::var("AIC_CACHE_DIR").ok().map(PathBuf::from);
}

impl Config {
    pub fn new() -> Result<Self, config::ConfigError> {
        let aic_cache_dir = get_aic_cache_dir();
        let builder = config::Config::builder()
            .set_default("aic_cache_dir", aic_cache_dir.to_str().unwrap())?;
        let cfg: Self = builder.build()?.try_deserialize()?;
        Ok(cfg)
    }
}

fn get_aic_cache_dir() -> PathBuf {
    let directory = if let Some(s) = AIC_CACHE_DIR.clone() {
        s
    } else if let Some(proj_dirs) = project_directory() {
        proj_dirs.data_local_dir().to_path_buf()
    } else {
        PathBuf::from(".").join(".cache")
    };
    directory
}

fn project_directory() -> Option<ProjectDirs> {
    ProjectDirs::from("com", "aic", env!("CARGO_PKG_NAME"))
}
