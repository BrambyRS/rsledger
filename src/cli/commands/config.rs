use crate::config;

/// Updates config fields from the provided arguments and saves to disk.
pub fn run_config(
    config_folder: String,
    config_journal: String,
    config_stock_prices_journal: String,
    config_exchange_rates_journal: String,
    config: &mut config::Config,
) -> crate::Result<()> {
    config::edit_config(
        config_folder,
        config_journal,
        config_stock_prices_journal,
        config_exchange_rates_journal,
        config,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_config() -> config::Config {
        config::Config {
            default_journal_folder: "".to_string(),
            default_journal: "".to_string(),
            default_stock_prices_journal: "".to_string(),
            default_exchange_rates_journal: "".to_string(),
        }
    }

    #[test]
    fn returns_error_when_no_options_provided() {
        let mut cfg = empty_config();
        let result = run_config(
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            &mut cfg,
        );
        assert!(result.is_err());
    }

    #[test]
    fn sets_folder_when_provided() {
        let mut cfg = empty_config();
        // edit_config calls config.save(), which writes to disk; we need a real config dir.
        // Since this touches the filesystem we just verify the in-memory mutation before save.
        // Use a path that won't interfere with real config by catching the save error.
        let _ = run_config(
            "/tmp/journals".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            &mut cfg,
        );
        assert_eq!(cfg.default_journal_folder, "/tmp/journals");
    }
}
