use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::Args;
use crate::config::Config;
use crate::transaction;

/// Prints `prompt` to stdout, flushes the buffer, reads a line from stdin,
/// and returns the trimmed result.
fn prompt_input(prompt: &str) -> io::Result<String> {
    print!("{prompt}");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

/// Creates a new, empty journal file at the path resolved from `args` and `config`.
/// Intermediate directories are created automatically if they do not exist.
pub fn new_journal(args: &Args, config: &Config) -> std::io::Result<()> {

    let journal_file: PathBuf = match get_journal_file_path(args, config) {
        Ok(path) => path,
        Err(e) => return Err(e),
    };

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

/// Interactively prompts the user for a date, description, and one or more postings,
/// then appends the resulting [`transaction::Transaction`] to the journal file.
///
/// Postings can be entered as:
/// - `<account>` — amount will be inferred (auto-balancing posting)
/// - `<account> <amount> <commodity>` — e.g. `expenses:food 50.00 SEK`
///
/// An empty line terminates posting input.
pub fn add_entry(args: &Args, config: &Config) -> std::io::Result<()> {
    // Get Journal path
    let journal_file: PathBuf = match get_journal_file_path(args, config) {
        Ok(path) => path,
        Err(e) => return Err(e),
    };

    if !journal_file.exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, format!("Journal file {} not found.", journal_file.display())));
    }
    
    println!("\nAdding entry to journal: {}", journal_file.display());
    println!("Enter postings on the format '<account> <amount> <commodity>'");
    println!("example: 'expenses:food 50.00 SEK') such that all are balanced.");
    println!("If you leave an amount blank, it will be inferred.");
    println!("Keep adding as many postings as you want, and then enter an empty line to finish the transaction.\n");
    let date_str: String = prompt_input("Date (YYYY-MM-DD): ")?;
    let description_str: String = prompt_input("Description: ")?;
    let mut postings: Vec<transaction::Posting> = Vec::new();

    loop {
        let posting_input: String = prompt_input("Posting: ")?;
        if posting_input.len() == 0 {
            break;
        }
        let parts: Vec<&str> = posting_input.split_whitespace().collect();
        if parts.len() == 1 {
            let account_str: String = parts[0].to_string();
            let amount: Option<transaction::commodity_value::CommodityValue> = None;

            postings.push(transaction::Posting::new(account_str, amount));
        } else if parts.len() == 3 {
            let account_str: String = parts[0].to_string();
            let amount_str: String = parts[1..].join(" ");
            let amount = match transaction::commodity_value::CommodityValue::from_str(&amount_str) {
                Ok(val) => Some(val),
                Err(_) => {
                    println!("Invalid amount format. Please enter a valid commodity amount (e.g. '50.00 SEK').");
                    continue;
                }
            };
            postings.push(transaction::Posting::new(account_str, amount));
        } else {
            println!("Invalid posting format. Please enter in the format '<account> <amount> <commodity>' (e.g. 'expenses:food 50.00 SEK') or '<account>' (e.g. 'assets:bank' for an auto-balancing posting).");
            continue;
        }
    }

    let entry: transaction::Transaction = transaction::Transaction::new(
        date_str,
        description_str,
        postings,
    );

    // Validate the transaction before writing to the journal
    if !entry.validate() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Transaction is not balanced. Please ensure that the amounts sum to zero."));
    }

    // Append entry to journal file
    let mut file = fs::OpenOptions::new().append(true).open(journal_file)?;
    write!(file, "{entry}")?;

    Ok(())
}

/// Resolves the journal file path from command-line arguments or the active config.
///
/// Returns the path from `--path` if supplied; otherwise constructs the path from
/// `config.default_journal_folder` and `config.default_journal`. Returns an error
/// if neither source provides a usable path.
fn get_journal_file_path(args: &Args, config: &Config) -> std::io::Result<PathBuf> {
    // Use the --path if it has been provided
    if args.journal_path.len() > 0 {
        return Ok(PathBuf::from(&args.journal_path));
    } else {
        // Otherwise, use the default journal from config
        if config.default_journal_folder.len() == 0 || config.default_journal.len() == 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "No journal path provided and default journal not set in config."));
        }
        return Ok(Path::new(&config.default_journal_folder).join(&config.default_journal));
    }
}
