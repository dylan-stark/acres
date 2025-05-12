#![deny(missing_docs)]

//! acres-cli is a simple CLI for accessing the Art Institute of Chicago's [public APIs].
//!
//! You can get the (first page of the) artworks listing with `acres-cli artworks`.
//!
//! [public APIs]: https://api.artic.edu/docs/#introduction

use clap::{Arg, Command, command, value_parser};
use eyre::Context;

#[doc(hidden)]
#[tokio::main]
async fn main() -> Result<(), eyre::Report> {
    let matches = command!()
        .propagate_version(true)
        .subcommand_required(true)
        .subcommand(
            Command::new("artworks")
                .about("The artworks collection")
                .arg(
                    Arg::new("ids")
                        .long("ids")
                        .help("comma-seperated list of artwork ids")
                        .value_delimiter(',')
                        .value_parser(value_parser!(u32)),
                )
                .arg(
                    Arg::new("limit")
                        .long("limit")
                        .help("max number of artworks to return at once")
                        .value_parser(value_parser!(u32)),
                )
                .arg(
                    Arg::new("page")
                        .long("page")
                        .help("which page to retrieve")
                        .value_parser(value_parser!(u32)),
                )
                .arg(
                    Arg::new("fields")
                        .long("fields")
                        .help("comma-separated list of fields to retrieve")
                        .value_parser(value_parser!(String)),
                )
                .arg(
                    Arg::new("include")
                        .long("include")
                        .help("comma-separated list of sub-resources to include")
                        .value_parser(value_parser!(String)),
                ),
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("artworks") {
        let api = aic::Api::new().artworks().list();
        let api = match matches.get_many::<u32>("ids") {
            Some(ids) => api.ids(ids.copied().collect()),
            None => api,
        };
        let api = match matches.get_one::<u32>("limit") {
            Some(limit) => api.limit(*limit),
            None => api,
        };
        let api = match matches.get_one::<u32>("page") {
            Some(page) => api.page(*page),
            None => api,
        };
        let api = match matches.get_many::<String>("fields") {
            Some(fields) => api.fields(fields.into_iter().map(|field| field.to_string()).collect()),
            None => api,
        };
        let api = match matches.get_many::<String>("include") {
            Some(include) => api.include(
                include
                    .into_iter()
                    .map(|include| include.to_string())
                    .collect(),
            ),
            None => api,
        };

        match api.get().await {
            Ok(listing) => println!("{}", listing),
            Err(error) => return Err(error).wrap_err("We couldn't get that listing ..."),
        }
    }

    Ok(())
}
