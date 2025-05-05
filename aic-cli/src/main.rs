#![deny(missing_docs)]

//! aic-cli is a simple CLI for accessing the Art Institute of Chicago's [public APIs].
//!
//! You can get the (first page of the) artworks listing with `aic-cli artworks`.
//!
//! [public APIs]: https://api.artic.edu/docs/#introduction

use clap::{command, value_parser, Arg, Command};

#[doc(hidden)]
#[tokio::main]
async fn main() {
    let matches = command!()
        .propagate_version(true)
        .subcommand_required(true)
        .subcommand(
            Command::new("artworks")
                .about("The artworks collection")
                .arg(
                    Arg::new("artwork-ids")
                        .long("ids")
                        .help("comma-seperated list of artwork ids")
                        .value_delimiter(',')
                        .value_parser(value_parser!(u32)),
                ),
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("artworks") {
        let api = aic::Api::new().artworks().list();
        let api = match matches.get_many::<u32>("artwork-ids") {
            Some(ids) => api.ids(ids.copied().collect()),
            None => api,
        };
        match api.get().await {
            Ok(listing) => println!("{}", listing),
            Err(error) => eprintln!("{:?}", error),
        }
    }
}
