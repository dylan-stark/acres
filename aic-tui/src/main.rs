// Following is meant to handle clippy bug about uneeded return
#![allow(clippy::needless_return)]

use clap::Parser;
use cli::Cli;
use color_eyre::Result;

use crate::app::App;

mod action;
mod aic;
mod app;
mod ascii_art;
mod cli;
mod components;
mod config;
mod errors;
mod logging;
mod tui;

#[tokio::main]
async fn main() -> Result<()> {
    crate::errors::init()?;
    crate::logging::init()?;

    let args = Cli::parse();
    let mut app = App::new(args.tick_rate, args.frame_rate, args.q)?;
    app.run().await?;

    Ok(())
}
