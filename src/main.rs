use clap::Parser;

fn main() -> anyhow::Result<()> {
    let cli = weavr::cli::Cli::parse();
    cli.run()
}
