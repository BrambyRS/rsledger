use clap::{Parser, Subcommand, ValueEnum};

mod cli_utils;
mod commodity_value;
mod config;
mod journalist;
mod price;
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
    Price {
        #[arg(
            short = 'e',
            long = "exchange-rate",
            help = "Add the entry to the default exchange rates journal file."
        )]
        exchange_rate: bool,

        #[arg(
            short = 'p',
            long = "price",
            help = "Add the entry to the default prices journal file."
        )]
        price: bool,
    },
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

enum DefaultJournalTypes {
    Transactions,
    ExchangeRates,
    Prices,
}

fn get_journal_file_path(
    path_arg: String,
    config: &config::Config,
    journal_type: DefaultJournalTypes,
) -> std::io::Result<std::path::PathBuf> {
    if path_arg.len() > 0 {
        Ok(std::path::PathBuf::from(&path_arg))
    } else {
        match journal_type {
            DefaultJournalTypes::Transactions => {
                if config.default_journal_folder.len() == 0 || config.default_journal.len() == 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "No journal path provided and default journal not set in config.",
                    ));
                } else {
                    return Ok(std::path::Path::new(&config.default_journal_folder)
                        .join(&config.default_journal));
                }
            }
            DefaultJournalTypes::ExchangeRates => {
                if config.default_journal_folder.len() == 0
                    || config.default_exchange_rates_journal.len() == 0
                {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "No journal path provided and default exchange rates journal not set in config.",
                    ));
                } else {
                    return Ok(std::path::Path::new(&config.default_journal_folder)
                        .join(&config.default_exchange_rates_journal));
                }
            }
            DefaultJournalTypes::Prices => {
                if config.default_journal_folder.len() == 0
                    || config.default_stock_prices_journal.len() == 0
                {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "No journal path provided and default stock prices journal not set in config.",
                    ));
                } else {
                    return Ok(std::path::Path::new(&config.default_journal_folder)
                        .join(&config.default_stock_prices_journal));
                }
            }
        }
    }
}

fn main() {
    // Parse input arguments
    let args: Args = Args::parse();

    // Load config
    let mut config: config::Config = config::Config::load();

    // Handle entry point
    match args.command {
        Command::New { open } => {
            // Resolve journal file path
            let journal_file: std::io::Result<std::path::PathBuf> = get_journal_file_path(
                args.journal_path,
                &config,
                DefaultJournalTypes::Transactions,
            );
            match journal_file {
                Err(e) => eprintln!("Error resolving journal file path: {}", e),
                Ok(path) => {
                    if let Err(e) = journalist::new_journal(&path, open) {
                        eprintln!("Error creating journal: {}", e);
                    }
                }
            }
        }
        Command::Add => {
            let journal_file: std::io::Result<std::path::PathBuf> = get_journal_file_path(
                args.journal_path,
                &config,
                DefaultJournalTypes::Transactions,
            );
            match journal_file {
                Err(e) => eprintln!("Error resolving journal file path: {}", e),
                Ok(path) => {
                    if let Err(e) = journalist::add_entry(&path) {
                        eprintln!("Error adding entry: {}", e);
                    }
                }
            }
        }
        Command::Price {
            exchange_rate,
            price,
        } => {
            if exchange_rate && price {
                eprintln!("Cannot be both exchange rate and price at the same time.");
            } else if exchange_rate {
                // Resolve journal file path
                let journal_file: std::io::Result<std::path::PathBuf> = get_journal_file_path(
                    args.journal_path,
                    &config,
                    DefaultJournalTypes::ExchangeRates,
                );
                match journal_file {
                    Err(e) => eprintln!("Error resolving journal file path: {}", e),
                    Ok(path) => {
                        if let Err(e) = journalist::add_price(&path) {
                            eprintln!("Error adding entry: {}", e);
                        }
                    }
                }
            } else if price {
                let journal_file: std::io::Result<std::path::PathBuf> =
                    get_journal_file_path(args.journal_path, &config, DefaultJournalTypes::Prices);
                match journal_file {
                    Err(e) => eprintln!("Error resolving journal file path: {}", e),
                    Ok(path) => {
                        if let Err(e) = journalist::add_price(&path) {
                            eprintln!("Error adding entry: {}", e);
                        }
                    }
                }
            } else {
                let journal_file: std::io::Result<std::path::PathBuf> = get_journal_file_path(
                    args.journal_path,
                    &config,
                    DefaultJournalTypes::Transactions,
                );
                match journal_file {
                    Err(e) => eprintln!("Error resolving journal file path: {}", e),
                    Ok(path) => {
                        if let Err(e) = journalist::add_price(&path) {
                            eprintln!("Error adding entry: {}", e);
                        }
                    }
                }
            }
        }
        Command::Import {
            csv_file,
            parser,
            rule_sheet,
        } => {
            let journal_file: std::io::Result<std::path::PathBuf> = get_journal_file_path(
                args.journal_path,
                &config,
                DefaultJournalTypes::Transactions,
            );
            match journal_file {
                Err(e) => eprintln!("Error resolving journal file path: {}", e),
                Ok(path) => {
                    let parser: Box<dyn journalist::csv_parser::CSVImporter> = match parser {
                        ParserOptions::Avanza => {
                            Box::new(journalist::csv_parser::avanza_parser::AvanzaParser::new())
                        }
                        ParserOptions::HSBCDebit => {
                            Box::new(journalist::csv_parser::default_parser::DefaultParser::new(
                                "assets:bank:hsbc".to_string(),
                                "GBP".to_string(),
                                std::path::PathBuf::from(&rule_sheet),
                                ',',
                                false,
                                0,
                                "%d/%m/%Y".to_string(),
                                vec![1],
                                2,
                                None,
                                Some(','),
                                '.',
                            ))
                        }
                        ParserOptions::HSBCCredit => {
                            Box::new(journalist::csv_parser::default_parser::DefaultParser::new(
                                "liabilities:credit:hsbc-credit-card".to_string(),
                                "GBP".to_string(),
                                std::path::PathBuf::from(&rule_sheet),
                                ',',
                                false,
                                0,
                                "%d/%m/%Y".to_string(),
                                vec![1],
                                2,
                                None,
                                Some(','),
                                '.',
                            ))
                        }
                        ParserOptions::SebDebit => {
                            Box::new(journalist::csv_parser::default_parser::DefaultParser::new(
                                "assets:bank:seb-lönekonto".to_string(),
                                "SEK".to_string(),
                                std::path::PathBuf::from(&rule_sheet),
                                ';',
                                true,
                                0,
                                "%Y-%m-%d".to_string(),
                                vec![3],
                                4,
                                None,
                                None,
                                '.',
                            ))
                        }
                        ParserOptions::SebSavings => {
                            Box::new(journalist::csv_parser::default_parser::DefaultParser::new(
                                "assets:bank:seb-sparkonto".to_string(),
                                "SEK".to_string(),
                                std::path::PathBuf::from(&rule_sheet),
                                ';',
                                true,
                                0,
                                "%Y-%m-%d".to_string(),
                                vec![3],
                                4,
                                None,
                                None,
                                '.',
                            ))
                        }
                        ParserOptions::Volksbank => {
                            Box::new(journalist::csv_parser::default_parser::DefaultParser::new(
                                "assets:bank:volksbank".to_string(),
                                "EUR".to_string(),
                                std::path::PathBuf::from(&rule_sheet),
                                ';',
                                true,
                                4,
                                "%d.%m.%Y".to_string(),
                                vec![6, 10],
                                11,
                                Some(12),
                                Some('.'),
                                ',',
                            ))
                        }
                    };

                    let csv_file = std::path::PathBuf::from(csv_file);

                    if let Err(e) = journalist::csv_parser::import_transactions_from_csv(
                        &*parser,
                        &csv_file,
                        &path,
                        &mut std::io::stdin().lock(),
                        &mut std::io::stdout(),
                    ) {
                        eprintln!("Error importing CSV: {}", e);
                    }
                }
            }
        }
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
