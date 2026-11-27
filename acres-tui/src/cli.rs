use clap::Parser;
use clap_stdin::FileOrStdin;

use crate::{
    config::{get_config_dir, get_data_dir},
    logging::LOG_FILE,
};

#[derive(Parser, Debug)]
#[command(version = version(), about)]
pub struct Cli {
    /// Explore a saved `Artworks` JSON, such as `acres-cli artworks` writes, or a saved list
    /// (`saved.json`), instead of the live collection. Searching then narrows it by title.
    ///
    /// Use `-` to pipe in from stdin.
    #[arg(long, value_name = "FILE or -")]
    pub from_file: Option<FileOrStdin<String>>,

    /// Answer only from the cache, without touching the network (or set `ACRES_OFFLINE=true`).
    #[arg(long)]
    pub offline: bool,

    /// Tick rate, i.e. number of ticks per second.
    #[arg(short, long, value_name = "FLOAT", default_value_t = 4.0)]
    pub tick_rate: f64,

    /// Frame rate, i.e. number of frames per second.
    #[arg(short, long, value_name = "FLOAT", default_value_t = 60.0)]
    pub frame_rate: f64,
}

/// The version, with the git description and build date when the build had them, then where
/// the TUI keeps its files.
pub fn version() -> String {
    let describe = env!("VERGEN_GIT_DESCRIBE");
    let build = match (describe.is_empty(), env!("VERGEN_BUILD_DATE")) {
        (true, date) => date.to_string(),
        (false, date) => format!("{describe}, {date}"),
    };
    let data_dir = get_data_dir();
    let cache_dir = acres::Catalog::builder()
        .build()
        .ok()
        .and_then(|catalog| catalog.cache_dir().map(|dir| dir.display().to_string()))
        .unwrap_or_else(|| "none (caching is off)".into());
    format!(
        "\
{version} ({build})

Config directory: {config}
Data directory: {data}
Log file: {log}
Cache directory: {cache_dir}",
        version = env!("CARGO_PKG_VERSION"),
        config = get_config_dir().display(),
        data = data_dir.display(),
        log = data_dir.join(LOG_FILE.as_str()).display(),
    )
}
