#![deny(missing_docs)]

//! acres-cli is a simple CLI for accessing the Art Institute of Chicago's [public APIs].
//!
//! You can get the (first page of the) artworks list with `acres-cli artworks`.
//!
//! [public APIs]: https://api.artic.edu/docs/#introduction

use acres::artworks;
use clap::{Arg, Command, command, value_parser};
use color_eyre::Result;
use eyre::Context;

mod logging;

#[doc(hidden)]
#[tokio::main]
async fn main() -> Result<()> {
    crate::logging::init()?;
    color_eyre::install()?;

    let matches = command!()
        .propagate_version(true)
        .subcommand_required(true)
        .subcommand(
            Command::new("artwork").about("An artwork").arg(
                Arg::new("id")
                    .help("the id of the artwork")
                    .required(true)
                    .value_parser(value_parser!(u32)),
            ),
        )
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

    if let Some(matches) = matches.subcommand_matches("artwork") {
        let id = matches
            .get_one::<u32>("id")
            .expect("clap `required` ensures its present");
        let artwork = artworks::Artwork::builder().id(*id);
        match artwork.build().await {
            Ok(artwork) => println!("{}", artwork),
            Err(error) => return Err(error).wrap_err("We couldn't get that artwork ..."),
        }
    }

    if let Some(matches) = matches.subcommand_matches("artworks") {
        let collection = artworks::Collection::builder();
        let collection = match matches.get_many::<u32>("ids") {
            Some(ids) => collection.ids(ids.copied().collect()),
            None => collection,
        };
        let collection = match matches.get_one::<u32>("limit") {
            Some(limit) => collection.limit(*limit),
            None => collection,
        };
        let collection = match matches.get_one::<u32>("page") {
            Some(page) => collection.page(*page),
            None => collection,
        };
        let collection = match matches.get_many::<String>("fields") {
            Some(fields) => {
                collection.fields(fields.into_iter().map(|field| field.to_string()).collect())
            }
            None => collection,
        };
        let collection = match matches.get_many::<String>("include") {
            Some(include) => collection.include(
                include
                    .into_iter()
                    .map(|include| include.to_string())
                    .collect(),
            ),
            None => collection,
        };

        match collection.build().await {
            Ok(collection) => println!("{}", collection),
            Err(error) => return Err(error).wrap_err("We couldn't get that list ..."),
        }
    }

    Ok(())
}
