use crate::{config::Config, file_entry::FileEntry, parser::parser_file_entry::ParserFileEntry};
use anyhow::{bail, Result};
use log::{debug, info};
use std::{
    fs::{self, DirEntry},
    path::Path,
};

pub struct VaultParser {
    config: Config,
}

impl VaultParser {
    pub const fn new(config: Config) -> Self {
        Self { config }
    }
    pub fn scan_vault(&self) -> Result<Vec<FileEntry>> {
        let mut tasks = vec![];
        info!("Scanning {:?}", self.config.vault_path);
        self.scan(&self.config.vault_path, &mut tasks)?;
        Ok(tasks)
    }

    fn scan(&self, path: &Path, tasks: &mut Vec<FileEntry>) -> Result<()> {
        if self.config.ignored_paths.contains(&path.to_owned()) {
            debug!("Ignoring {path:?} (ignored_paths list)");
            return Ok(());
        }

        let entries = path.read_dir()?;
        for entry_err in entries {
            if let Ok(entry) = entry_err {
                let name = entry.file_name().into_string().unwrap();
                if self.config.ignore_dot_files && name.starts_with('.') {
                    debug!("Ignoring {name:?} (dot file)");
                    continue;
                }
                if entry.path().is_dir() {
                    // recursive call for this subdir
                    self.scan(&entry.path(), tasks)?;
                } else if let Some(file_tasks) = self.parse_file(&entry) {
                    tasks.push(file_tasks);
                }
            } else {
                bail!("Error while reading an entry from {:?}", path)
            }
        }

        Ok(())
    }

    fn parse_file(&self, entry: &DirEntry) -> Option<FileEntry> {
        debug!("Parsing {:?}", entry.file_name());
        let content = fs::read_to_string(entry.path()).unwrap_or_default();
        let parser = ParserFileEntry {
            config: &self.config,
        };

        parser.parse_file(&entry.path(), &content.as_str())
    }
}
