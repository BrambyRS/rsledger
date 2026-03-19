use dirs;
use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use toml;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub default_journal_folder: String,
    pub default_journal: String,
}

impl Config {
    pub fn load() -> Self {
        // 
        let config_dir: PathBuf = match dirs::config_dir() {
            Some(dir) => dir,
            None => panic!("Could not determine config directory."),
        };

        let config_file = config_dir.join("rsledger").join("config.toml");

        if config_file.exists() {
            let config_str: String = std::fs::read_to_string(config_file).expect("Failed to read config file.");
            return toml::from_str(&config_str).expect("Failed to parse config file.")
        } else {
            // If the config file doesn't exist, return a default config.
            return Config {
                default_journal_folder: "".to_string(),
                default_journal: "".to_string(),
            };
        }
    }

    pub fn save(&self) {
        let config_dir: PathBuf = match dirs::config_dir() {
            Some(dir) => dir,
            None => panic!("Could not determine config directory."),
        };

        let config_file = config_dir.join("rsledger").join("config.toml");

        // Create the directory if it doesn't exist
        if let Some(parent) = config_file.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).expect("Failed to create config directory.");
            }
        }

        let config_str = toml::to_string(self).expect("Failed to serialize config.");
        std::fs::write(config_file, config_str).expect("Failed to write config file.");
    }

    pub fn set_default_journal_folder(&mut self, folder: String) {
        self.default_journal_folder = folder;
    }

    pub fn set_default_journal(&mut self, journal: String) {
        self.default_journal = journal;
    }
}

pub fn edit_config(config_folder: String, config_journal: String, config: &mut Config) -> std::io::Result<()> {
    // Check that at least one of the config options is provided
    if config_folder.len() == 0 && config_journal.len() == 0 {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "At least one config option must be provided."));
    }

    if config_folder.len() > 0 {
        config.set_default_journal_folder(config_folder.clone());
    }

    if config_journal.len() > 0 {
        config.set_default_journal(config_journal.clone());
    }

    config.save();
    Ok(())
}