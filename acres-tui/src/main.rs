#![deny(missing_docs)]

//! acres-tui is a terminal UI for exploring artworks from the Art Institute of Chicago's
//! [public APIs](https://api.artic.edu/docs/).

use std::process::ExitCode;

use acres_tui::{app::App, cli::Cli, entry::Entry, failure};
use clap::Parser;
use color_eyre::Result;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        // The terminal is restored by now, so the report goes to stderr as the CLI writes it.
        Err(report) => {
            let failure = failure::at_exit(report.as_ref());
            if std::env::var("RUST_BACKTRACE").is_ok_and(|v| v != "0") {
                eprintln!("{report:?}\n");
            }
            eprint!("{}", failure.to_text());
            ExitCode::from(failure.exit_code)
        }
    }
}

async fn run() -> Result<()> {
    acres_tui::errors::init()?;
    acres_tui::logging::init()?;

    let args = Cli::parse();
    let from_file = match args.from_file {
        Some(file) => Some(Entry::from_text(&file.contents()?)?),
        None => None,
    };
    let mut app = App::new(args.tick_rate, args.frame_rate, from_file, args.offline)?;
    app.run().await?;

    Ok(())
}
