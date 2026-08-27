# rsledger

A Rust CLI tool for importing bank and broker CSV exports into [hledger](https://hledger.org)-compatible plain-text journal files.

## Features

- **Regex rule sheets** — classify transactions automatically via TOML rule files (assign accounts or skip entries)
- **Deduplication** — compares incoming transactions against the existing journal by hash, skipping exact duplicates and prompting user to manually deduplicate on partial matches.
- **Price directives** — record commodity prices and exchange rates; import them from Avanza positions exports
- **No floating point** — all amounts use fixed-precision decimal arithmetic

## Installation

```bash
./build_and_install.sh   # runs tests, builds release binary, installs to /opt/rsledger
```

Or manually:

```bash
cargo build --release
```

## Usage

```bash
# Set up default journal location
rsledger config -f ~/journals -j main.journal

# Create a new journal
rsledger new

# Import transactions from a CSV
rsledger import transactions.csv seb-debit --rule-sheet rules.toml

# Interactively add a single transaction
rsledger add

# Add a price directive
rsledger price -p

# Import prices from an Avanza positions CSV
rsledger import-prices positions.csv
```

### Supported parsers

`avanza`, `hsbc-debit`, `hsbc-credit`, `seb-debit`, `seb-savings`, `volksbank`

### Rule sheets

Create a `.toml` file to auto-classify transactions during import:

```toml
[[rules]]
pattern = "^GROCERY STORE"
action = "assign_account"
account = "expenses:food:groceries"

[[rules]]
pattern = "^INTERNAL TRANSFER"
action = "skip"
```

Pass it with `--rule-sheet <path>` on the `import` command.

## Configuration

Stored at `~/.config/rsledger/config.toml`:

```toml
default_journal_folder = "/path/to/journals"
default_journal = "main.journal"
default_stock_prices_journal = "prices.journal"
default_exchange_rates_journal = "exchange_rates.journal"
```

## See Also

See [AGENTS.md](AGENTS.md) for a full architectural overview, module map, and design decisions.
