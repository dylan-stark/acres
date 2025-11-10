#![deny(missing_docs)]

//! acres-cli is a simple CLI for accessing the Art Institute of Chicago's [public APIs].
//!
//! [public APIs]: https://api.artic.edu/docs/#introduction

use std::io::{self, Write};

use acres::{
    AcresError, Api, Cached,
    artworks::{self, Manifest, request::artwork},
};
use clap::{Arg, Command, command, value_parser};
use clap_stdin::FileOrStdin;
use color_eyre::{
    Result,
    eyre::{Report, WrapErr},
};
use image_to_ascii_builder::{
    Alphabet, Ascii, BrightnessOffset, CharWidth, ConversionAlgorithm, Font, Metric,
};

#[doc(hidden)]
mod logging;

#[doc(hidden)]
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
                        .help("image file or '-' to read image bytes from stdin")
                        .value_parser(value_parser!(FileOrStdin)),
                )
                .arg(
                    Arg::new("alphabet")
                        .long("alphabet")
                        .help("alphabet to use")
                        .default_value(Alphabet::default().to_string())
                        .value_parser(value_parser!(Alphabet)),
                )
                .arg(
                    Arg::new("brightness-offset")
                        .long("brightness-offset")
                        .help("brightness offset")
                        .default_value(BrightnessOffset::default().to_string())
                        .value_parser(value_parser!(BrightnessOffset)),
                )
                .arg(
                    Arg::new("conversion-algorithm")
                        .long("conversion-algorithm")
                        .help("alphabet to use")
                        .default_value(ConversionAlgorithm::default().to_string())
                        .value_parser(value_parser!(ConversionAlgorithm)),
                )
                .arg(
                    Arg::new("font")
                        .long("font")
                        .help("font to use")
                        .default_value(Font::default().to_string())
                        .value_parser(value_parser!(Font)),
                )
                .arg(
                    Arg::new("metric")
                        .long("metric")
                        .help("the metric to use")
                        .default_value(Metric::default().to_string())
                        .value_parser(value_parser!(Metric)),
                )
                .arg(
                    Arg::new("width")
                        .long("width")
                        .help("how many characters wide")
                        .default_value(CharWidth::default().to_string())
                        .value_parser(value_parser!(CharWidth)),
                ),
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
                        .default_value("full")
                        .value_parser(value_parser!(iiif::Region)),
                )
                .arg(
                    Arg::new("size")
                        .long("size")
                        .help("dimensions to which the extracted region is to be scaled")
                        .default_value("843,")
                        .value_parser(value_parser!(iiif::Size)),
                )
                .arg(
                    Arg::new("rotation")
                        .long("rotation")
                        .help("mirroring and rotation")
                        .default_value("0.0")
                        .value_parser(value_parser!(iiif::Rotation)),
                )
                .arg(
                    Arg::new("quality")
                        .long("quality")
                        .help("whether image is delivered in color, grayscale, or black-and-white")
                        .default_value("default")
                        .value_parser(value_parser!(iiif::Quality)),
                )
                .arg(
                    Arg::new("format")
                        .long("format")
                        .help("format of the returned image")
                        .default_value("jpg")
                        .value_parser(value_parser!(iiif::Format)),
                )
                .arg(Arg::new("to").long("to").help("type of output").default_value("url").value_parser(value_parser!(IiifTo)))
        )
        .subcommand(
            Command::new("iiif-info")
                .about("Get associated IIIF image information")
                .arg(
                    Arg::new("artwork")
                        .required(true)
                        .help("artwork JSON file or '-' to read JSON from stdin")
                        .value_parser(value_parser!(FileOrStdin)),
                )
                .arg(Arg::new("to").long("to").help("type of output").default_value("url").value_parser(value_parser!(IiifTo)))
                .subcommand(Command::new("info").about("Retrieve image information.")),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("artwork", matches)) => {
            let api = Api::new();
            let id = matches
                .get_one::<u32>("id")
                .copied()
                .expect("clap ensures this is provided");
            let request = artwork::Request::new(api.base_uri(), id);
            let artwork: Cached = Api::new().fetch(request.to_string()).await?;
            println!("{}", artwork)
        }
        Some(("artwork-manifest", matches)) => {
            let api = Api::new();
            let id = matches
                .get_one::<u32>("id")
                .copied()
                .expect("clap ensures id is provided");
            let request = artworks::request::manifest::Request::new(api.base_uri(), id);
            let manifest: Manifest = Api::new().fetch(request.to_string()).await?;
            println!("{}", manifest)
        }
        Some(("artworks", matches)) => {
            let api = Api::new();
            match artworks::request::artworks::Request::builder()
                .base_uri(api.base_uri())
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
            {
                Ok(request) => {
                    let collection: Cached = api.fetch(request.to_string()).await?;
                    println!("{}", collection)
                }
                Err(error) => return Err(error).wrap_err("We couldn't get that list ..."),
            }
        }
        Some(("artworks-search", matches)) => {
            let api = Api::new();
            match artworks::Search::builder()
                .base_uri(api.base_uri())
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
            {
                Ok(request) => {
                    let search: Cached = api.fetch(request.to_string()).await?;
                    println!("{}", search)
                }
                Err(error) => return Err(error).wrap_err("We couldn't complete that search ..."),
            }
        }
        Some(("ascii-art", matches)) => {
            let image_reader = matches
                .get_one::<FileOrStdin>("image")
                .expect("clap ensures we get the bytes")
                .clone()
                .into_reader()
                .context("failed to clone file-or-stdin")?;
            let art = Ascii::builder()
                .input_reader(image_reader)
                .context("failed to read input image")?
                .alphabet(
                    matches
                        .get_one::<Alphabet>("alphabet")
                        .cloned()
                        .expect("at least default set"),
                )
                .brightness_offset(
                    matches
                        .get_one::<BrightnessOffset>("brightness-offset")
                        .cloned()
                        .expect("at least default set"),
                )
                .conversion_algorithm(
                    matches
                        .get_one::<ConversionAlgorithm>("conversion-algorithm")
                        .cloned()
                        .expect("at least default set"),
                )
                .chars_wide(
                    matches
                        .get_one::<CharWidth>("width")
                        .cloned()
                        .expect("at least default set"),
                )
                .font(
                    matches
                        .get_one::<Font>("font")
                        .cloned()
                        .expect("at least default set"),
                )
                .metric(
                    matches
                        .get_one::<Metric>("metric")
                        .cloned()
                        .expect("at least default set"),
                )
                .build()
                .context("failed to build art")?;
            println!("{}\n", art);
        }
        Some(("iiif", matches)) => {
            let artwork = artworks::ArtworkInfo::load(
                matches
                    .get_one::<FileOrStdin>("artwork")
                    .expect("clap ensures this is a string")
                    .clone()
                    .into_reader()?,
            )
            .ok_or(AcresError::LoadArtworkInfo)?;
            let base_uri: iiif::Uri = artwork.try_into()?;
            let image_request = iiif::ImageRequest::builder()
                .uri(base_uri)
                .region(
                    matches
                        .get_one::<iiif::Region>("region")
                        .cloned()
                        .expect("at least default set"),
                )
                .size(
                    matches
                        .get_one::<iiif::Size>("size")
                        .cloned()
                        .expect("at least default set"),
                )
                .rotation(
                    matches
                        .get_one::<iiif::Rotation>("rotation")
                        .cloned()
                        .expect("at least default set"),
                )
                .quality(
                    matches
                        .get_one::<iiif::Quality>("quality")
                        .cloned()
                        .expect("at least default set"),
                )
                .format(
                    matches
                        .get_one::<iiif::Format>("format")
                        .cloned()
                        .expect("at least default set"),
                )
                .build();
            match matches.get_one::<IiifTo>("to") {
                Some(IiifTo::Url) => println!("{}", image_request),
                Some(IiifTo::Bytes) => {
                    let response: bytes::Bytes =
                        Api::new().fetch(image_request.to_string()).await?;
                    io::stdout()
                        .write_all(&response)
                        .context("failed to write image bytes")?;
                }
                None => unreachable!("default value means we shouldn't get here"),
            }
        }
        Some(("iiif-info", matches)) => {
            let artwork = artworks::ArtworkInfo::load(
                matches
                    .get_one::<FileOrStdin>("artwork")
                    .expect("clap ensures this is a string")
                    .clone()
                    .into_reader()?,
            )
            .ok_or(AcresError::LoadArtworkInfo)?;
            let request: iiif::InformationRequest = iiif::Uri::try_from(artwork)?.into();
            let response: bytes::Bytes = Api::new().fetch(request.to_string()).await?;
            io::stdout()
                .write_all(&response)
                .context("failed to write json bytes")?;
        }
        _ => unreachable!("clap should ensure we don't get here"),
    };

    Ok(())
}
