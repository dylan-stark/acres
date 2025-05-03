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
        Resource::Artworks => {
            let listing = aic::Api::new().artworks().await;
            println!("{}", listing);
        }
    }
}
