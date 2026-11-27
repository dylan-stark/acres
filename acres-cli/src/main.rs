#![deny(missing_docs)]

//! acres-cli is a simple CLI for accessing the Art Institute of Chicago's [public APIs].
//!
//! [public APIs]: https://api.artic.edu/docs/#introduction

use std::{
    io::{self, IsTerminal, Write},
    process::ExitCode,
};

use acres::{
    Catalog, Resource,
    artworks::{self, request::artwork},
};
use clap::{Arg, Command, command, value_parser};
use clap_stdin::FileOrStdin;
use color_eyre::{
    Result,
    eyre::{Report, WrapErr},
};
#[doc(hidden)]
mod ascii_art;
#[doc(hidden)]
mod logging;
#[doc(hidden)]
mod report;

use report::{ErrorFormat, Outcome};

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
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let format =
        ErrorFormat::from_args_and_env(&args, std::env::var("ACRES_ERROR_FORMAT").ok().as_deref());
    let setup = crate::logging::init().and_then(|()| color_eyre::install());
    if let Err(report) = setup {
        return fail(&report, format);
    }

    let matches = match cli().try_get_matches_from(&args) {
        Ok(matches) => matches,
        // `--help` and `--version` also come back as errors, written to stdout.
        Err(error) if !error.use_stderr() || format == ErrorFormat::Text => {
            let _ = error.print();
            return ExitCode::from(u8::try_from(error.exit_code()).unwrap_or(report::exit::USAGE));
        }
        Err(error) => return report::emit(&report::usage(&error), format),
    };
    match run(matches).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(report) => fail(&report, format),
    }
}

/// Reports `report` on stderr and picks the exit code (see [`report`]).
#[doc(hidden)]
fn fail(report: &Report, format: ErrorFormat) -> ExitCode {
    match report::classify(report) {
        Outcome::BrokenPipe => ExitCode::SUCCESS,
        Outcome::Failed(failure) => {
            let backtrace = std::env::var("RUST_BACKTRACE").is_ok_and(|v| v != "0");
            if backtrace && format == ErrorFormat::Text {
                eprintln!("{report:?}\n");
            }
            report::emit(&failure, format)
        }
    }
}

/// The command line.
#[doc(hidden)]
fn cli() -> Command {
    command!()
        .propagate_version(true)
        .subcommand_required(true)
        .after_help(EXIT_CODES)
        .arg(
            Arg::new("error-format")
                .long("error-format")
                .global(true)
                .value_name("FORMAT")
                .value_parser(value_parser!(ErrorFormat))
                .help("how errors are written to stderr: text, or one line of JSON for programs (also ACRES_ERROR_FORMAT)"),
        )
        .arg(
            Arg::new("offline")
                .long("offline")
                .global(true)
                .action(clap::ArgAction::SetTrue)
                .help("answer only from the cache (also ACRES_OFFLINE=true)"),
        )
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
                    Arg::new("mode")
                        .long("mode")
                        .help("ASCII characters, or half or quadrant block glyphs in full color")
                        .default_value("ascii")
                        .value_parser(value_parser!(ascii_art::Mode)),
                )
                .arg(
                    Arg::new("ramp")
                        .long("ramp")
                        .help(format!(
                            "preset ({}) or characters from darkest to brightest (ascii mode)",
                            charaoke::Ramp::PRESETS
                                .iter()
                                .map(|(name, _)| *name)
                                .collect::<Vec<_>>()
                                .join(", ")
                        ))
                        .default_value("alphabet")
                        .value_parser(ascii_art::parse_ramp),
                )
                .arg(
                    Arg::new("brightness-offset")
                        .long("brightness-offset")
                        .help("darkens the art by subtracting this from each cell's brightness (ascii mode)")
                        .default_value("0")
                        .value_parser(value_parser!(u8)),
                )
                .arg(
                    Arg::new("width")
                        .long("width")
                        .help("how many characters wide [default: terminal width, or 80]")
                        .value_parser(value_parser!(u32).range(1..)),
                )
                .arg(
                    Arg::new("color")
                        .long("color")
                        .help("color output")
                        .default_value("auto")
                        .value_parser(value_parser!(ascii_art::ColorChoice)),
                )
                .arg(
                    Arg::new("timings")
                        .long("timings")
                        .help("print how long each stage took to stderr")
                        .action(clap::ArgAction::SetTrue),
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
}

/// The exit codes, listed under `--help`.
#[doc(hidden)]
const EXIT_CODES: &str = "\
Exit codes:
  0  success
  1  other failure
  2  usage: the command line could not be parsed
  3  not found: the API has no such record
  4  offline miss: offline, and the cache has no entry
  5  transient: rate limited, network trouble or an API server error; try again later
  6  invalid input: a bad request, IIIF parameter or input file
  7  environment: configuration, the cache directory, or reading and writing files
  8  bad response: the API answered with something acres can't read";

/// Runs the parsed command.
#[doc(hidden)]
async fn run(matches: clap::ArgMatches) -> Result<()> {
    let mut catalog = Catalog::builder();
    if matches.get_flag("offline") {
        catalog = catalog.offline(true);
    }
    let catalog = catalog.build()?;

    match matches.subcommand() {
        Some(("artwork", matches)) => {
            let id = matches
                .get_one::<u32>("id")
                .copied()
                .expect("clap ensures this is provided");
            let request = artwork::Request::new(catalog.base_uri(), id);
            print_body(&catalog.raw(request, Resource::Record).await?.value)?;
        }
        Some(("artwork-manifest", matches)) => {
            let id = matches
                .get_one::<u32>("id")
                .copied()
                .expect("clap ensures id is provided");
            let request = artworks::request::manifest::Request::new(catalog.base_uri(), id);
            print_body(&catalog.raw(request, Resource::Record).await?.value)?;
        }
        Some(("artworks", matches)) => {
            match artworks::request::artworks::Request::builder()
                .base_uri(catalog.base_uri())
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
                Ok(request) => print_body(&catalog.raw(request, Resource::Page).await?.value)?,
                Err(error) => return Err(error).wrap_err("could not build the artworks request"),
            }
        }
        Some(("artworks-search", matches)) => {
            match artworks::request::search::Request::builder()
                .base_uri(catalog.base_uri())
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
                Ok(request) => print_body(&catalog.raw(request, Resource::Page).await?.value)?,
                Err(error) => return Err(error).wrap_err("could not build the search request"),
            }
        }
        Some(("ascii-art", matches)) => {
            let image_reader = open_input(
                matches
                    .get_one::<FileOrStdin>("image")
                    .expect("clap ensures we get the bytes"),
            )?;
            let ramp = matches
                .get_one::<charaoke::Ramp>("ramp")
                .cloned()
                .expect("at least default set")
                .with_brightness_offset(
                    *matches
                        .get_one::<u8>("brightness-offset")
                        .expect("at least default set"),
                );
            let width = matches
                .get_one::<u32>("width")
                .copied()
                .unwrap_or_else(ascii_art::default_width);
            let depth = matches
                .get_one::<ascii_art::ColorChoice>("color")
                .expect("at least default set")
                .depth(io::stdout().is_terminal(), |key| std::env::var(key).ok());
            let options = ascii_art::Options {
                mode: *matches
                    .get_one::<ascii_art::Mode>("mode")
                    .expect("at least default set"),
                ramp,
                width: std::num::NonZeroU32::new(width).expect("clap ensures at least 1"),
                depth,
                timings: matches.get_flag("timings"),
            };
            ascii_art::run(image_reader, &mut io::stdout().lock(), options)?;
        }
        Some(("iiif", matches)) => {
            let artwork = artworks::ArtworkInfo::load(open_input(
                matches
                    .get_one::<FileOrStdin>("artwork")
                    .expect("clap ensures this is a string"),
            )?)?;
            let base_uri: iiif::BaseUri = artwork.try_into()?;
            let image_request = iiif::ImageRequest::builder()
                .base_uri(base_uri)
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
                Some(IiifTo::Url) => {
                    writeln!(io::stdout(), "{image_request}").context("failed to write the URL")?
                }
                Some(IiifTo::Bytes) => {
                    let image = catalog.image(&image_request).await?.value;
                    io::stdout()
                        .write_all(&image)
                        .context("failed to write image bytes")?;
                }
                None => unreachable!("default value means we shouldn't get here"),
            }
        }
        Some(("iiif-info", matches)) => {
            let artwork = artworks::ArtworkInfo::load(open_input(
                matches
                    .get_one::<FileOrStdin>("artwork")
                    .expect("clap ensures this is a string"),
            )?)?;
            let request: iiif::InformationRequest = iiif::BaseUri::try_from(artwork)?.into();
            print_body(&catalog.raw(request, Resource::Record).await?.value)?;
        }
        _ => unreachable!("clap should ensure we don't get here"),
    };

    Ok(())
}

/// Opens a file argument, or stdin for `-`, naming it if that fails.
#[doc(hidden)]
fn open_input(input: &FileOrStdin) -> Result<impl io::Read> {
    let name = if input.is_stdin() {
        "stdin".to_string()
    } else {
        input.filename().to_string()
    };
    input
        .clone()
        .into_reader()
        .with_context(|| format!("could not open {name}"))
}

/// Writes a JSON body to stdout as the API sent it, ending with a newline.
#[doc(hidden)]
fn print_body(body: &[u8]) -> Result<()> {
    let mut stdout = io::stdout().lock();
    stdout
        .write_all(body)
        .context("failed to write the response")?;
    if !body.ends_with(b"\n") {
        stdout
            .write_all(b"\n")
            .context("failed to write the response")?;
    }
    Ok(())
}
