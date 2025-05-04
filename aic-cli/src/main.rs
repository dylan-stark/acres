use clap::Parser;

#[derive(Clone, clap::ValueEnum)]
enum Resource {
    Artworks,
}

#[derive(Parser)]
struct Cli {
    resource: Resource,
}

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
