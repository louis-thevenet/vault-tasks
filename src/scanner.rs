use crate::core::FileEntry;
use crate::{config::Config, parser::file_entry};
use anyhow::{bail, Result};
use log::{debug, info};
use std::{
    fs::{self, DirEntry},
    path::Path,
};

pub struct Scanner {
    config: Config,
}

impl Scanner {
    pub const fn new(config: Config) -> Self {
        Self { config }
    }
    pub fn scan_vault(self) -> Result<Vec<FileEntry>> {
        let mut tasks = vec![];
        info!("Scanning {:?}", self.config.vault_path);
        self.scan(&self.config.vault_path, &mut tasks)?;
        Ok(tasks)
    }

    fn scan(&self, path: &Path, tasks: &mut Vec<FileEntry>) -> Result<()> {
        let entries = path.read_dir()?;
        for entry_err in entries {
            if let Ok(entry) = entry_err {
                let name = entry.file_name().into_string().unwrap();
                if name.starts_with('.') {
                    continue;
                }
                if entry.path().is_dir() {
                    // recursive call for this subdir
                    self.scan(&entry.path(), tasks)?;
                } else if let Some(file_tasks) = self.parse_file(&entry) {
                    debug!("Tasks found in {name}:\n{file_tasks}");
                    tasks.push(file_tasks);
                }
            } else {
                bail!("Error while reading an entry from {:?}", path)
            }
        }

        Ok(())
    }

    fn parse_file(&self, entry: &DirEntry) -> Option<FileEntry> {
        let content = fs::read_to_string(entry.path()).unwrap_or_default();
        let parser = file_entry::ParserFileEntry {
            config: &self.config,
        };

        parser.parse_file(
            entry
                .file_name()
                .into_string()
                .unwrap_or_else(|_| "Couldn't read filename".to_string()),
            &content.as_str(),
        )
    }
}
