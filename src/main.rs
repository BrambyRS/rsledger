use clap::Parser;

mod journalist;

#[derive(Parser)]
#[command(version, about = "Plain text CLI accounting tool inspired by hledger.", long_about = None)]
struct Args {
    // Entry point
    entry_point: String,

    // Journal path for new journal creation or entry addition
    #[arg(short, long, default_value = "main.journal")]
    journal_path: String,
}

fn main() {
    // Parse input arguments
    let args: Args = Args::parse();

    // Handle entry point
    match args.entry_point.as_str() {
        "new" => {
            if let Err(e) = journalist::new_journal(&args) {
                eprintln!("Error creating journal: {}", e);
            }
        }
        "add" => {
            if let Err(e) = journalist::add_entry(&args) {
                eprintln!("Error adding entry: {}", e);
            }
        }
        _ => eprintln!("Unknown entry point: {}", args.entry_point)
    }
}
