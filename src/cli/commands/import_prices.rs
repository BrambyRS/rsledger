use crate::journalist::writer::prices_importer;

/// Imports prices from a positions CSV file into the journal.
pub fn run_import_prices(
    journal_file: &std::path::PathBuf,
    csv_file: &std::path::PathBuf,
) -> crate::Result<()> {
    prices_importer::import_prices(csv_file, journal_file)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn csv_path(filename: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test")
            .join("csvs")
            .join(filename)
    }

    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    struct TempJournal(std::path::PathBuf);

    impl TempJournal {
        fn new_empty() -> Self {
            let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let path =
                std::env::temp_dir().join(format!("rsledger_import_prices_test_{}.journal", id));
            std::fs::write(&path, "").unwrap();
            TempJournal(path)
        }
        fn path(&self) -> &std::path::PathBuf {
            &self.0
        }
    }

    impl Drop for TempJournal {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    #[test]
    fn imports_prices_from_positions_csv() {
        let tmp = TempJournal::new_empty();
        run_import_prices(tmp.path(), &csv_path("2026-01-15_positions.csv")).unwrap();
        let contents = std::fs::read_to_string(tmp.path()).unwrap();
        assert!(!contents.is_empty(), "expected price directives to be imported");
    }
}
