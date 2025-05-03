use clap::Parser;

#[derive(Clone, clap::ValueEnum)]
enum Resource {
    Artworks,
}

#[derive(Parser)]
struct Cli {
    resource: Resource,
}

fn main() {
    let args = Cli::parse();

    match args.resource {
        Resource::Artworks => {
            let listing = aic::Api::artworks();
            println!("{}", listing);
        }
    }
}
