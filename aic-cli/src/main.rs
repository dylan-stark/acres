#![warn(missing_docs)]

//! aic-cli is a simple CLI for accessing the Art Institute of Chicago's [public APIs].
//!
//! You can get the (first page of the) artworks listing with `aic-cli artworks`.
//!
//! [public APIs]: https://api.artic.edu/docs/#introduction

use clap::Parser;

#[doc(hidden)]
#[derive(Clone, clap::ValueEnum)]
enum Resource {
    Artworks,
}

#[doc(hidden)]
#[derive(Parser)]
struct Cli {
    resource: Resource,
}

#[doc(hidden)]
#[tokio::main]
async fn main() {
    let args = Cli::parse();

    match args.resource {
        Resource::Artworks => match aic::Api::new().artworks().await {
            Ok(listing) => println!("{}", listing),
            Err(error) => eprintln!("{:?}", error),
        },
    }
}
