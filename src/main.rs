use clap::Parser;

mod config;
mod journalist;

#[derive(Parser)]
#[command(version, about = "Plain text CLI accounting tool inspired by hledger.", long_about = None)]
struct Args {
    // Entry point
    entry_point: String,

    // Options related to journal file and configuration
    #[arg(short = 'p', long = "path", default_value = "", help = "Path to the journal file to use.")]
    journal_path: String,

    #[arg(short = 'f', long = "folder", default_value = "", help = "Journal folder to set as default.")]
    config_folder: String,

    #[arg(short = 'j', long = "journal", default_value = "main.journal", help = "File name of journal file in default folder to use.")]
    config_journal: String,
}

fn main() {
    // Parse input arguments
    let args: Args = Args::parse();

    // Load config
    let mut config: config::Config = config::Config::load();

    // Handle entry point
    match args.entry_point.as_str() {
        "new" => {
            if let Err(e) = journalist::new_journal(&args, &config) {
                eprintln!("Error creating journal: {}", e);
            }
        }
        "add" => {
            if let Err(e) = journalist::add_entry(&args, &config) {
                eprintln!("Error adding entry: {}", e);
            }
        }
        "config" => {
            if let Err(e) = config::edit_config(&args, &mut config) {
                eprintln!("Error editing config: {}", e);
            }
        }
        _ => eprintln!("Unknown entry point: {}", args.entry_point)
    }
}
