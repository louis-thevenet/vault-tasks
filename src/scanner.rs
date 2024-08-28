use crate::config::Config;
use anyhow::{bail, Result};
use log::info;
use std::path::Path;

pub struct Scanner {
    config: Config,
}

impl Scanner {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
    pub fn scan_vault(self) -> Result<()> {
        self.scan(&self.config.vault_path)
    }

    fn scan(&self, path: &Path) -> Result<()> {
        let entries = path.read_dir()?;
        for entry_err in entries {
            if let Ok(entry) = entry_err {
                let name = entry.file_name().into_string().unwrap();
                if name.starts_with(".") {
                    info!("Skipping {name}");
                    continue;
                }
                if entry.path().is_dir() {
                    info!("Opening directory {:?}", entry.file_name());
                    self.scan(&entry.path())?;
                } else {
                    info!("Reading {name}");
                }
            } else {
                bail!("Error while reading an entry from {:?}", path)
            }
        }

        Ok(())
    }
}
