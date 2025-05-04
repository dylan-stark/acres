#![deny(missing_docs)]

//! aic-cli is a simple CLI for accessing the Art Institute of Chicago's [public APIs].
//!
//! You can get the (first page of the) artworks listing with `aic-cli artworks`.
//!
//! [public APIs]: https://api.artic.edu/docs/#introduction

use clap::{command, Command};

#[doc(hidden)]
#[tokio::main]
async fn main() {
    let matches = command!()
        .propagate_version(true)
        .subcommand_required(true)
        .subcommand(Command::new("artworks").about("The artworks collection"))
        .get_matches();

    if matches.subcommand_matches("artworks").is_some() {
        match aic::Api::new().artworks().await {
            Ok(listing) => println!("{}", listing),
            Err(error) => eprintln!("{:?}", error),
        }
    }
}
