use clap::{Parser, Subcommand, ValueEnum};

#[derive(ValueEnum, Clone)]
pub enum ParserOptions {
    Avanza,
    HSBCDebit,
    HSBCCredit,
    SebDebit,
    SebSavings,
    Volksbank,
}

#[derive(Subcommand)]
pub enum Command {
    New {
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
            help = "Path to a .toml file containing classification rules to apply when importing the transactions. If not provided, no classification rules will be applied.",
            default_value = ""
        )]
        rule_sheet: String,
    },
    ImportPrices {
        #[arg(help = "Positions CSV file to import prices from.")]
        csv_file: String,
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
pub struct Args {
    #[command(subcommand, help = "Entry point to execute.")]
    pub command: Command,

    #[arg(
        short = 'p',
        long = "path",
        default_value = "",
        help = "Path to the journal file to use."
    )]
    pub journal_path: String,
}
