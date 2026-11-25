#![deny(missing_docs)]

//! acres-tui is a terminal UI for exploring artworks from the Art Institute of Chicago's
//! [public APIs](https://api.artic.edu/docs/).

use acres_tui::{app::App, cli::Cli, entry::Entry};
use clap::Parser;
use color_eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    acres_tui::errors::init()?;
    acres_tui::logging::init()?;

    let args = Cli::parse();
    let from_file = match args.from_file {
        Some(file) => Some(Entry::from_file(file.contents()?)),
        None => None,
    };
    let mut app = App::new(args.tick_rate, args.frame_rate, from_file, args.offline)?;
    app.run().await?;

    Ok(())
}
