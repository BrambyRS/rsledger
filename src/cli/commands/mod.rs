pub mod add;
pub mod config;
pub mod import;
pub mod import_prices;
pub mod new;
pub mod price;

use crate::cli::args::{Args, Command};
use crate::config as app_config;

use std::io::{BufRead, Write};

enum DefaultJournalType {
    Transactions,
    ExchangeRates,
    Prices,
}

fn get_journal_file_path(
    path_arg: &str,
    config: &app_config::Config,
    journal_type: DefaultJournalType,
) -> crate::Result<std::path::PathBuf> {
    if !path_arg.is_empty() {
        return Ok(std::path::PathBuf::from(path_arg));
    }
    match journal_type {
        DefaultJournalType::Transactions => {
            if config.default_journal_folder.is_empty() || config.default_journal.is_empty() {
                Err(crate::error::RsledgerError::CliError(
                    "No journal path provided and default journal not set in config.".to_string(),
                ))
            } else {
                Ok(std::path::Path::new(&config.default_journal_folder)
                    .join(&config.default_journal))
            }
        }
        DefaultJournalType::ExchangeRates => {
            if config.default_journal_folder.is_empty()
                || config.default_exchange_rates_journal.is_empty()
            {
                Err(crate::error::RsledgerError::CliError(
                    "No journal path provided and default exchange rates journal not set in config."
                        .to_string(),
                ))
            } else {
                Ok(std::path::Path::new(&config.default_journal_folder)
                    .join(&config.default_exchange_rates_journal))
            }
        }
        DefaultJournalType::Prices => {
            if config.default_journal_folder.is_empty()
                || config.default_stock_prices_journal.is_empty()
            {
                Err(crate::error::RsledgerError::CliError(
                    "No journal path provided and default stock prices journal not set in config."
                        .to_string(),
                ))
            } else {
                Ok(std::path::Path::new(&config.default_journal_folder)
                    .join(&config.default_stock_prices_journal))
            }
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
            let path = get_journal_file_path(
                &args.journal_path,
                &config,
                DefaultJournalType::Transactions,
            )?;
            new::run_new(&path, open, reader, writer)
        }
        Command::Add => {
            let path = get_journal_file_path(
                &args.journal_path,
                &config,
                DefaultJournalType::Transactions,
            )?;
            add::run_add(&path, reader, writer)
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
            let path =
                get_journal_file_path(&args.journal_path, &config, journal_type)?;
            crate::cli::commands::price::run_price(&path, reader, writer)
        }
        Command::Import {
            csv_file,
            parser,
            rule_sheet,
            accept_partial_matches,
        } => {
            let path = get_journal_file_path(
                &args.journal_path,
                &config,
                DefaultJournalType::Transactions,
            )?;
            import::run_import(
                &path,
                &std::path::PathBuf::from(&csv_file),
                parser,
                &rule_sheet,
                accept_partial_matches,
                reader,
                writer,
            )
        }
        Command::ImportPrices { csv_file } => {
            let path = get_journal_file_path(
                &args.journal_path,
                &config,
                DefaultJournalType::Prices,
            )?;
            import_prices::run_import_prices(&path, &std::path::PathBuf::from(&csv_file))
        }
        Command::Config {
            config_folder,
            config_journal,
            config_stock_prices_journal,
            config_exchange_rates_journal,
        } => config::run_config(
            config_folder,
            config_journal,
            config_stock_prices_journal,
            config_exchange_rates_journal,
            &mut config,
        ),
    }
}
