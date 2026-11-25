#![deny(missing_docs)]

//! acres-tui is a terminal UI for exploring artworks from the Art Institute of Chicago's
//! [public APIs](https://api.artic.edu/docs/).

use acres_tui::{app::App, cli::Cli};
use clap::Parser;
use color_eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    acres_tui::errors::init()?;
    acres_tui::logging::init()?;

    let args = Cli::parse();
    let mut app = App::new(args.tick_rate, args.frame_rate, args.artworks.contents()?)?;
    app.run().await?;

    Ok(())
}
