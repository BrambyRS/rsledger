use clap::{Parser, Subcommand, ValueEnum};

mod cli_utils;
mod config;
mod journalist;
mod transaction;

#[derive(ValueEnum, Clone)]
enum ParserOptions {
    Avanza,
    HSBCDebit,
    HSBCCredit,
    SebDebit,
    SebSavings,
    Volksbank,
}

#[derive(Subcommand)]
enum Command {
    New {
        // When creating a new journal, also add an opening transaction with the current date.
        #[arg(
            long = "open",
            help = "When creating a new journal, also add an opening transaction with the current date."
        )]
        open: bool,
    },
    Add,
    Import {
        #[arg(help = "CSV file to import from.")]
        csv_file: String,

        #[arg(help = "Parser logic to use when importing the CSV file.")]
        parser: ParserOptions,

        #[arg(
            long = "rule-sheet",
            help = "Path to a .toml file containing classification rulest to apply when importing the transactions. If not provided, no classification rules will be applied.",
            default_value = ""
        )]
        rule_sheet: String,
    },
    Config {
        #[arg(
            short = 'f',
            long = "folder",
            default_value = "",
            help = "Journal folder to set as default."
        )]
        config_folder: String,

        #[arg(
            short = 'j',
            long = "journal",
            default_value = "main.journal",
            help = "File name of journal file in default folder to use."
        )]
        config_journal: String,

        #[arg(
            short = 's',
            long = "stock-prices-journal",
            default_value = "stock_prices.journal",
            help = "File name of journal file in default folder to use for stock prices."
        )]
        config_stock_prices_journal: String,

        #[arg(
            short = 'e',
            long = "exchange-rates-journal",
            default_value = "exchange_rates.journal",
            help = "File name of journal file in default folder to use for exchange rates."
        )]
        config_exchange_rates_journal: String,
    },
}

#[derive(Parser)]
#[command(version, about = "Plain text CLI accounting tool inspired by hledger.", long_about = None)]
struct Args {
    #[command(subcommand, help = "Entry point to execute.")]
    command: Command,

    // Options related to journal file and configuration
    #[arg(
        short = 'p',
        long = "path",
        default_value = "",
        help = "Path to the journal file to use."
    )]
    journal_path: String,
}

fn get_journal_file_path(
    args: &Args,
    config: &config::Config,
) -> std::io::Result<std::path::PathBuf> {
    if args.journal_path.len() > 0 {
        Ok(std::path::PathBuf::from(&args.journal_path))
    } else {
        if config.default_journal_folder.len() == 0 || config.default_journal.len() == 0 {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No journal path provided and default journal not set in config.",
            ))
        } else {
            Ok(std::path::Path::new(&config.default_journal_folder).join(&config.default_journal))
        }
    }
}

fn main() {
    // Parse input arguments
    let args: Args = Args::parse();

    // Load config
    let mut config: config::Config = config::Config::load();

    // Resolve journal file path
    let journal_file: std::io::Result<std::path::PathBuf> = get_journal_file_path(&args, &config);

    // Handle entry point
    match args.command {
        Command::New { open } => match journal_file {
            Err(e) => eprintln!("Error resolving journal file path: {}", e),
            Ok(path) => {
                if let Err(e) = journalist::new_journal(&path, open) {
                    eprintln!("Error creating journal: {}", e);
                }
            }
        },
        Command::Add => match journal_file {
            Err(e) => eprintln!("Error resolving journal file path: {}", e),
            Ok(path) => {
                if let Err(e) = journalist::add_entry(&path) {
                    eprintln!("Error adding entry: {}", e);
                }
            }
        },
        Command::Import {
            csv_file,
            parser,
            rule_sheet,
        } => match journal_file {
            Err(e) => eprintln!("Error resolving journal file path: {}", e),
            Ok(path) => {
                let parser: Box<dyn journalist::csv_parser::CSVImporter> = match parser {
                    ParserOptions::Avanza => {
                        Box::new(journalist::csv_parser::avanza_parser::AvanzaParser::new())
                    }
                    ParserOptions::HSBCDebit => {
                        Box::new(journalist::csv_parser::hsbc_parser::HSBCParser::new(
                            "assets:bank:hsbc".to_string(),
                            std::path::PathBuf::from(&rule_sheet),
                        ))
                    }
                    ParserOptions::HSBCCredit => {
                        Box::new(journalist::csv_parser::hsbc_parser::HSBCParser::new(
                            "liabilities:credit:hsbc-credit-card".to_string(),
                            std::path::PathBuf::from(&rule_sheet),
                        ))
                    }
                    ParserOptions::SebDebit => {
                        Box::new(journalist::csv_parser::seb_parser::SebParser::new(
                            "assets:bank:seb-lönekonto".to_string(),
                            std::path::PathBuf::from(&rule_sheet),
                        ))
                    }
                    ParserOptions::SebSavings => {
                        Box::new(journalist::csv_parser::seb_parser::SebParser::new(
                            "assets:bank:seb-sparkonto".to_string(),
                            std::path::PathBuf::from(&rule_sheet),
                        ))
                    }
                    ParserOptions::Volksbank => Box::new(
                        journalist::csv_parser::volksbank_parser::VolksbankParser::new(
                            "assets:bank:volksbank".to_string(),
                            std::path::PathBuf::from(&rule_sheet),
                        ),
                    ),
                };

                let csv_file = std::path::PathBuf::from(csv_file);

                if let Err(e) =
                    journalist::csv_parser::import_transactions_from_csv(&*parser, &csv_file, &path)
                {
                    eprintln!("Error importing CSV: {}", e);
                }
            }
        },
        Command::Config {
            config_folder,
            config_journal,
            config_stock_prices_journal,
            config_exchange_rates_journal,
        } => {
            if let Err(e) = config::edit_config(
                config_folder,
                config_journal,
                config_stock_prices_journal,
                config_exchange_rates_journal,
                &mut config,
            ) {
                eprintln!("Error editing config: {}", e);
            }
            config.save();
        }
    }
}
