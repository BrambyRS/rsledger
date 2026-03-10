use clap::Parser;
use std::fs;
use std::path::Path;
use std::io::{self, Write};

#[derive(Parser)]
#[command(version, about = "Plain text CLI accounting tool inspired by hledger.", long_about = None)]
struct Args {
    // Entry point
    entry_point: String,

    // Journal path for new journal creation or entry addition
    #[arg(short, long, default_value = "main.journal")]
    journal_path: String,
}

// TODO: Set default config
fn new_journal(args: &Args) -> std::io::Result<()> {
    let journal_file = Path::new(&args.journal_path);

    // Create the directory if it doesn't exist
    if let Some(parent) = journal_file.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    // Create an empty journal file
    fs::File::create(journal_file)?;

    return Ok(());
}

// TODO: Input validation, error handling, multi currency support, multi entry support, etc.
fn add_entry(args: &Args) -> std::io::Result<()> {
    // Get Journal path
    let journal_file = Path::new(&args.journal_path);
    if !journal_file.exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, format!("Journal file {} not found.", journal_file.display())));
    }
    
    // Get date
    print!("Date (YYY-MM-DD): ");
    io::stdout().flush()?;
    let mut date = String::new();
    io::stdin().read_line(&mut date)?;
    let date: &str = date.trim();

    // Get description
    print!("Description: ");
    io::stdout().flush()?;
    let mut description = String::new();
    io::stdin().read_line(&mut description)?;
    let description: &str = description.trim();

    // Get account from
    print!("From Account: ");
    io::stdout().flush()?;
    let mut from_account = String::new();
    io::stdin().read_line(&mut from_account)?;
    let from_account: &str = from_account.trim();

    // Get amount from
    print!("Amount: ");
    io::stdout().flush()?;
    let mut amount_from = String::new();
    io::stdin().read_line(&mut amount_from)?;
    let amount_from: &str = amount_from.trim();

    // Get account to
    print!("To Account: ");
    io::stdout().flush()?;
    let mut to_account = String::new();
    io::stdin().read_line(&mut to_account)?;
    let to_account: &str = to_account.trim();

    // Get amount to
    print!("Amount: ");
    io::stdout().flush()?;
    let mut amount_to = String::new();
    io::stdin().read_line(&mut amount_to)?;
    let amount_to: &str = amount_to.trim();

    // Append entry to journal file
    let entry: String = format!("{date} {description}\n\t{from_account} {amount_from}\n\t{to_account} {amount_to}\n\n");
    fs::OpenOptions::new().append(true).open(journal_file)?;
    fs::write(journal_file, entry)?;

    Ok(())
}

fn main() {
    // Parse input arguments
    let args: Args = Args::parse();

    // Handle entry point
    match args.entry_point.as_str() {
        "new" => {
            if let Err(e) = new_journal(&args) {
                eprintln!("Error creating journal: {}", e);
            }
        }
        "add" => {
            if let Err(e) = add_entry(&args) {
                eprintln!("Error adding entry: {}", e);
            }
        }
        _ => eprintln!("Unknown entry point: {}", args.entry_point)
    }
}
