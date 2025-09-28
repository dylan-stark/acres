#![deny(missing_docs)]

//! acres-cli is a simple CLI for accessing the Art Institute of Chicago's [public APIs].
//!
//! [public APIs]: https://api.artic.edu/docs/#introduction

use std::io::{self, Write};

use acres::{AcresError, artworks};
use clap::{Arg, Command, command, value_parser};
use clap_stdin::FileOrStdin;
use color_eyre::{
    Result, Section,
    eyre::{Report, WrapErr},
};
use eyre::ContextCompat;

#[doc(hidden)]
mod logging;

#[derive(clap::ValueEnum, Clone, Default)]
enum IiifTo {
    #[default]
    #[value(name = "url")]
    Url,
    #[value(name = "bytes")]
    Bytes,
}

#[doc(hidden)]
#[tokio::main]
async fn main() -> Result<(), Report> {
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
        )
        .subcommand(
            Command::new("artwork-manifest").about("Retrieve the manifest for this artwork")
                .arg(
                    Arg::new("id")
                        .help("the id of the artwork")
                        .required(true)
                        .value_parser(value_parser!(u32)),
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
                .arg(
                    Arg::new("q")
                        .long("q")
                        .help("search query")
                        .value_parser(value_parser!(String)),
                )
                .arg(
                    Arg::new("query")
                        .long("query")
                        .help("complex query (in Elasticsearch domain syntax)")
                        .value_parser(value_parser!(String)),
                )
                .arg(
                    Arg::new("sort")
                        .long("sort")
                        .help("sort one or more fields")
                        .value_parser(value_parser!(String)),
                )
                .arg(
                    Arg::new("from")
                        .long("from")
                        .help("starting point of results")
                        .value_parser(value_parser!(u32)),
                )
                .arg(
                    Arg::new("size")
                        .long("size")
                        .help("number of results to return")
                        .value_parser(value_parser!(u32)),
                )
                .arg(
                    Arg::new("facets")
                    .long("facets")
                    .help( "comman-separated list of 'count' aggregation facets to include in results")
                    .value_delimiter(',')
                    .value_parser(value_parser!(String))
                ),
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
                        .help("rectangular portion of the full image to be returned")
                        .value_parser(iiif::Region::parse),
                )
                .arg(
                    Arg::new("size")
                        .long("size")
                        .help("dimensions to which the extracted region is to be scaled")
                        .value_parser(iiif::Size::parse),
                )
                .arg(
                    Arg::new("rotation")
                        .long("rotation")
                        .help("mirroring and rotation")
                        .value_parser(iiif::Rotation::parse),
                )
                .arg(
                    Arg::new("quality")
                        .long("quality")
                        .help("whether image is delivered in color, grayscale, or black-and-white")
                        .value_parser(iiif::Quality::parse),
                )
                .arg(
                    Arg::new("format")
                        .long("format")
                        .help("format of the returned image")
                        .value_parser(iiif::Format::parse),
                )
                .arg(Arg::new("to").long("to").help("type of output").default_value("url").value_parser(value_parser!(IiifTo)))
                .subcommand(Command::new("info").about("Retrieve image information.")),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("artwork", matches)) => match artworks::Artwork::builder()
            .id(matches.get_one::<u32>("id").copied())
            .build()
            .await
        {
            Ok(artwork) => println!("{}", artwork),
            Err(error) => return Err(error).wrap_err("We couldn't get that artwork ..."),
        },
        Some(("artwork-manifest", matches)) => {
            match artworks::Manifest::builder()
                .id(matches.get_one::<u32>("id").copied())
                .build()
                .await
            {
                Ok(manifest) => println!("{}", manifest),
                Err(error) => return Err(error).wrap_err("We couldn't get that manifest ..."),
            }
        }
        Some(("artworks", matches)) => {
            match artworks::Collection::builder()
                .ids(
                    matches
                        .get_many::<u32>("ids")
                        .map(|ids| ids.copied().collect::<Vec<u32>>()),
                )
                .limit(matches.get_one::<u32>("limit").copied())
                .page(matches.get_one::<u32>("page").copied())
                .fields(
                    matches
                        .get_many::<String>("fields")
                        .map(|fields| fields.cloned().collect()),
                )
                .include(
                    matches
                        .get_many::<String>("include")
                        .map(|include| include.cloned().collect()),
                )
                .build()
                .await
            {
                Ok(collection) => println!("{}", collection),
                Err(error) => return Err(error).wrap_err("We couldn't get that list ..."),
            }
        }
        Some(("artworks-search", matches)) => {
            match artworks::Search::builder()
                .q(matches.get_one::<String>("q").cloned())
                .query(matches.get_one::<String>("query").cloned())
                .sort(matches.get_one::<String>("sort").cloned())
                .from(matches.get_one::<u32>("from").cloned())
                .size(matches.get_one::<u32>("size").cloned())
                .facets(
                    matches
                        .get_many::<String>("facets")
                        .map(|facets| facets.cloned().collect()),
                )
                .build()
                .await
            {
                Ok(search) => println!("{}", search),
                Err(error) => return Err(error).wrap_err("We couldn't complete that search ..."),
            }
        }
        Some(("iiif", matches)) => {
            let artwork = artworks::ArtworkInfo::load(
                matches
                    .get_one::<FileOrStdin>("artwork")
                    .expect("clap ensures this is a string")
                    .clone()
                    .into_reader()?,
            )
            .ok_or(AcresError::ArtworkError(
                "not able to read that artwork info".to_string(),
            ))?;
            let base_uri = iiif::BaseUri::builder()
                .scheme(
                    iiif::Scheme::parse(artwork.config.iiif_url.scheme())
                        .map_err(AcresError::IiifError)?,
                )
                .server(
                    artwork
                        .config
                        .iiif_url
                        .host_str()
                        .context("failed to parse host from URL")?,
                )
                .prefix(artwork.config.iiif_url.path())
                .identifier(&artwork.data.image_id)
                .build()
                .map_err(|error| AcresError::IiifError(error.to_string()))?;
            match matches.subcommand() {
                None => {
                    match iiif::ImageRequest::builder()
                        .base_uri(base_uri)
                        .region(matches.get_one::<iiif::Region>("region").cloned())
                        .size(matches.get_one::<iiif::Size>("size").cloned())
                        .rotation(matches.get_one::<iiif::Rotation>("rotation").cloned())
                        .quality(matches.get_one::<iiif::Quality>("quality").cloned())
                        .format(matches.get_one::<iiif::Format>("format").cloned())
                        .build()
                        .await
                    {
                        Ok(iiif) => match matches.get_one::<IiifTo>("to") {
                            Some(IiifTo::Url) => println!("{}", iiif),
                            Some(IiifTo::Bytes) => {
                                let bytes = reqwest::get(iiif.to_string())
                                    .await
                                    .wrap_err("Oh, no! Couldn't process that image request.")?
                                    .error_for_status()
                                    .wrap_err("Oh, no! Error status code returned.")
                                    .suggestion(
                                        "Make sure provided settings are supported \
                                for this image. Try rerunning with `acres iiif [...] \
                                info` to find out what's supported.",
                                    )?
                                    .bytes()
                                    .await
                                    .wrap_err("Oh, no! Couldn't get them image bytes.")?;
                                let mut stdout = io::stdout();
                                stdout
                                    .write_all(&bytes)
                                    .context("failed to write image bytes")?;
                            }
                            None => unreachable!("default value means we shouldn't get here"),
                        },
                        Err(error) => return Err(error).wrap_err("Oops, something went wrong ..."),
                    }
                }
                Some(("info", _)) => {
                    let iiif = iiif::InformationRequest::new(base_uri);
                    let json = reqwest::get(iiif.to_string())
                        .await
                        .wrap_err("Oh, no! Couldn't process that image request.")?
                        .error_for_status()
                        .wrap_err("Oh, no! Error status code returned.")?
                        .bytes()
                        .await
                        .wrap_err("Oh, no! Couldn't get response data.")?;
                    let mut stdout = io::stdout();
                    stdout
                        .write_all(&json)
                        .context("failed to write json bytes")?;
                }
                _ => unreachable!("should not be able to reach this point"),
            }
        }
        _ => unreachable!("clap should ensure we don't get here"),
    };

    Ok(())
}
