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
    Cli::parse();
}
