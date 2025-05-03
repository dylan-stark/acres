use std::{env, path::PathBuf};

use clap::Parser;
use directories::ProjectDirs;
use lazy_static::lazy_static;
use serde::Deserialize;

#[derive(Clone, clap::ValueEnum)]
enum Resource {
    Artworks,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct Config {
    #[serde(default)]
    aic_cache_dir: PathBuf,
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

#[derive(Parser)]
struct Cli {
    resource: Resource,
}

fn main() {
    let config = Config::new().unwrap();
    Cli::parse();

    let artworks_json_path = config.aic_cache_dir.join("artworks.json");
    let artworks_json = if artworks_json_path.is_file() {
        std::fs::read_to_string(artworks_json_path).unwrap()
    } else {
        String::from(
            r#"
        {
            "pagination": {
                "total": 128194
            }
        }"#,
        )
    };
    println!("{}", artworks_json);
}
