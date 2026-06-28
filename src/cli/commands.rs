pub mod add;
pub mod config;
pub mod import;
pub mod import_prices;
pub mod new;
pub mod price;

use crate::cli::args::{Args, Command};
use crate::config as app_config;
use crate::journal;

use std::io::{BufRead, Write};

enum DefaultJournalType {
    Transactions,
    ExchangeRates,
    Prices,
}

fn get_journal_file(
    path_arg: &str,
    config: &app_config::Config,
    journal_type: DefaultJournalType,
) -> crate::Result<journal::JournalFile> {
    if !path_arg.is_empty() {
        return Ok(journal::JournalFile::new(std::path::PathBuf::from(
            path_arg,
        )));
    }
    match journal_type {
        DefaultJournalType::Transactions => {
            if config.default_journal_folder.is_empty() || config.default_journal.is_empty() {
                return Err(crate::error::RsledgerError::CliError(
                    "No journal path provided and default journal not set in config.".to_string(),
                ));
            }
            return Ok(journal::JournalFile::new(
                std::path::Path::new(&config.default_journal_folder).join(&config.default_journal),
            ));
        }
        DefaultJournalType::ExchangeRates => {
            if config.default_journal_folder.is_empty()
                || config.default_exchange_rates_journal.is_empty()
            {
                return Err(crate::error::RsledgerError::CliError(
                    "No journal path provided and default exchange rates journal not set in config."
                        .to_string(),
                ));
            }
            return Ok(journal::JournalFile::new(
                std::path::Path::new(&config.default_journal_folder)
                    .join(&config.default_exchange_rates_journal),
            ));
        }
        DefaultJournalType::Prices => {
            if config.default_journal_folder.is_empty()
                || config.default_stock_prices_journal.is_empty()
            {
                return Err(crate::error::RsledgerError::CliError(
                    "No journal path provided and default stock prices journal not set in config."
                        .to_string(),
                ));
            }
            return Ok(journal::JournalFile::new(
                std::path::Path::new(&config.default_journal_folder)
                    .join(&config.default_stock_prices_journal),
            ));
        }
    }
}

/// Routes the parsed CLI arguments to the appropriate command handler.
/// Returns `Err` on any failure so that `main` can print the error and exit.
pub fn dispatch(
    args: Args,
    mut config: app_config::Config,
    reader: &mut impl BufRead,
    writer: &mut impl Write,
) -> crate::Result<()> {
    match args.command {
        Command::New { open } => {
            let jf = match get_journal_file(
                &args.journal_path,
                &config,
                DefaultJournalType::Transactions,
            ) {
                Ok(jf) => jf,
                Err(e) => return Err(e),
            };
            return new::run_new(jf, open, reader, writer);
        }
        Command::Add => {
            let jf = match get_journal_file(
                &args.journal_path,
                &config,
                DefaultJournalType::Transactions,
            ) {
                Ok(jf) => jf,
                Err(e) => return Err(e),
            };
            return add::run_add(jf, reader, writer);
        }
        Command::Price {
            exchange_rate,
            price,
        } => {
            if exchange_rate && price {
                return Err(crate::error::RsledgerError::CliError(
                    "Cannot be both exchange rate and price at the same time.".to_string(),
                ));
            }
            let journal_type = if exchange_rate {
                DefaultJournalType::ExchangeRates
            } else if price {
                DefaultJournalType::Prices
            } else {
                DefaultJournalType::Transactions
            };
            let jf = match get_journal_file(&args.journal_path, &config, journal_type) {
                Ok(jf) => jf,
                Err(e) => return Err(e),
            };
            return crate::cli::commands::price::run_price(jf, reader, writer);
        }
        Command::Import {
            csv_file,
            parser,
            rule_sheet,
            accept_partial_matches,
        } => {
            let jf = match get_journal_file(
                &args.journal_path,
                &config,
                DefaultJournalType::Transactions,
            ) {
                Ok(jf) => jf,
                Err(e) => return Err(e),
            };
            return import::run_import(
                jf,
                &std::path::PathBuf::from(&csv_file),
                parser,
                &rule_sheet,
                accept_partial_matches,
                reader,
                writer,
            );
        }
        Command::ImportPrices { csv_file } => {
            let jf = match get_journal_file(&args.journal_path, &config, DefaultJournalType::Prices)
            {
                Ok(jf) => jf,
                Err(e) => return Err(e),
            };
            return import_prices::run_import_prices(jf, &std::path::PathBuf::from(&csv_file));
        }
        Command::Config {
            config_folder,
            config_journal,
            config_stock_prices_journal,
            config_exchange_rates_journal,
        } => {
            match config::run_config(
                config_folder,
                config_journal,
                config_stock_prices_journal,
                config_exchange_rates_journal,
                &mut config,
            ) {
                Ok(_) => {}
                Err(e) => return Err(e),
            }
            config.save();
            return Ok(());
        }
    }
}
