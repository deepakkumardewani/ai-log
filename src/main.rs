use clap::Parser;

fn main() -> anyhow::Result<()> {
    let cli = cclog::cli::Cli::parse();
    cli.run()
}
