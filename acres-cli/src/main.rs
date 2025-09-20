#![deny(missing_docs)]

//! acres-cli is a simple CLI for accessing the Art Institute of Chicago's [public APIs].
//!
//! [public APIs]: https://api.artic.edu/docs/#introduction

use std::io::Write;

use acres::artworks;
use clap::{Arg, Command, command, value_parser};
use clap_stdin::FileOrStdin;
use color_eyre::Result;
use crossterm::terminal;
use eyre::Context;

#[doc(hidden)]
mod logging;

#[doc(hidden)]
#[derive(clap::ValueEnum, Clone)]
enum AsOption {
    Iiif,
    Jpeg,
    Json,
    Ascii,
}

#[doc(hidden)]
#[tokio::main]
async fn main() -> Result<()> {
    crate::logging::init()?;
    color_eyre::install()?;

    let matches = command!()
        .propagate_version(true)
        .subcommand_required(true)
        .subcommand(
            Command::new("artwork")
                .about("Retrieve a piece of artwork")
                .arg(
                    Arg::new("id")
                        .help("the id of the artwork")
                        .required(true)
                        .value_parser(value_parser!(u32)),
                )
                .subcommand(
                    Command::new("manifest").about("Retrieve the manifest for this artwork"),
                ),
        )
        .subcommand(
            Command::new("artworks")
                .about("List artworks collection")
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
        .subcommand(
            Command::new("artworks-search")
                .about("Search the artworks collection")
                .arg(Arg::new("q").long("q").help("search query"))
                .arg(
                    Arg::new("query")
                        .long("query")
                        .help("complex query (in Elasticsearch domain syntax)"),
                )
                .arg(
                    Arg::new("sort")
                        .long("sort")
                        .help("sort one or more fields"),
                )
                .arg(
                    Arg::new("from")
                        .long("from")
                        .help("starting point of results"),
                )
                .arg(
                    Arg::new("size")
                        .long("size")
                        .help("number of results to return"),
                )
                .arg(Arg::new("facets").long("facets").help(
                    "comman-separated list of 'count' aggregation facets to include in results",
                )),
        )
        .subcommand(
            Command::new("ascii-art")
                .about("Work with ASCII art")
                .arg(
                    Arg::new("image")
                        .required(true)
                        .help("image file or '-' to read JSON from stdin")
                        .value_parser(value_parser!(FileOrStdin)),
                )
                .arg(
                    Arg::new("width")
                        .long("width")
                        .help("how many characters wide"),
                )
                .arg(Arg::new("from").long("from").help("type of input")),
        )
        .subcommand(
            Command::new("iiif")
                .about("Work with IIIF URLs")
                .arg(
                    Arg::new("artwork")
                        .required(true)
                        .help("artwork JSON file or '-' to read JSON from stdin")
                        .value_parser(value_parser!(FileOrStdin)),
                )
                .arg(
                    Arg::new("region")
                        .long("region")
                        .help("rectangular portion of the full image to be returned"),
                )
                .arg(
                    Arg::new("size")
                        .long("size")
                        .help("dimensions to which the extracted region is to be scaled"),
                )
                .arg(
                    Arg::new("rotation")
                        .long("rotation")
                        .help("mirroring and rotation"),
                )
                .arg(
                    Arg::new("quality")
                        .long("quality")
                        .help("whether image is delivered in color, grayscale, or black-and-white"),
                )
                .arg(
                    Arg::new("format")
                        .long("format")
                        .help("format of the returned image"),
                )
                .arg(Arg::new("to").long("to").help("type of output")),
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("artwork") {
        let id = matches
            .get_one::<u32>("id")
            .expect("clap `required` ensures its present");
        let artwork = artworks::Artwork::builder().id(*id);
        match artwork.build().await {
            Ok(artwork) => match matches
                .get_one::<AsOption>("as")
                .expect("clap ensures this is a string")
            {
                AsOption::Ascii => {
                    let chars_wide = match matches.get_one::<usize>("width") {
                        Some(&width) => width,
                        None => match terminal::size() {
                            Ok((columns, _)) => columns.into(),
                            Err(_) => 80,
                        },
                    };
                    match artwork.to_ascii(chars_wide).await {
                        Ok(ascii) => std::io::stdout()
                            .write_all((ascii + "\n").as_bytes())
                            .wrap_err("We failed writing out the ASCII ...")?,
                        Err(error) => {
                            return Err(error).wrap_err("We couldn't generate that ASCII art ...");
                        }
                    }
                }
                AsOption::Iiif => match artwork.to_iiif() {
                    Ok(iiif_url) => println!("{}", iiif_url),
                    Err(error) => {
                        return Err(error).wrap_err("We couldn't generate that IIIF url ...");
                    }
                },
                AsOption::Jpeg => match artwork.to_image().await {
                    Ok(image) => std::io::stdout()
                        .write_all(&image)
                        .wrap_err("We failed writing out the image ...")?,
                    Err(error) => {
                        return Err(error).wrap_err("We couldn't get that image ...");
                    }
                },
                AsOption::Json => println!("{}", artwork),
            },
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
        tracing::debug!(?collection);

        match collection.build().await {
            Ok(collection) => println!("{}", collection),
            Err(error) => return Err(error).wrap_err("We couldn't get that list ..."),
        }
    }

    if let Some(matches) = matches.subcommand_matches("iiif") {
        let artwork = matches
            .get_one::<FileOrStdin>("artwork")
            .expect("clap ensures this is a string");
        println!("{:?}", artwork.clone().contents())
    }

    Ok(())
}
