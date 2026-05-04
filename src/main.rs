use clap::Parser;

mod cli;
mod commodity_value;
mod config;
mod error;
mod journalist;
mod price;
mod transaction;

/// Default Result type using RsledgerError
type Result<T> = std::result::Result<T, crate::error::RsledgerError>;

fn main() {
    let args = cli::args::Args::parse();
    let config = config::Config::load();
    if let Err(e) = cli::commands::dispatch(
        args,
        config,
        &mut std::io::stdin().lock(),
        &mut std::io::stdout(),
    ) {
        eprintln!("Error: {}", e);
    }
}
