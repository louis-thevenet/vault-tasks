use color_eyre::{Result, eyre::bail};
use std::{
    fs::{self, DirEntry},
    path::Path,
};
use tracing::{debug, info};

use crate::{TasksConfig, parser::parser_file_entry::ParserFileEntry};

use super::vault_data::VaultData;

pub struct VaultParser {
    config: TasksConfig,
}

impl VaultParser {
    pub const fn new(config: TasksConfig) -> Self {
        Self { config }
    }
    pub fn scan_vault(&self) -> Result<VaultData> {
        let mut tasks = VaultData::Directory(
            self.config.core.vault_path.to_str().unwrap().to_owned(),
            vec![],
        );
        info!("Scanning {:?}", self.config.core.vault_path);
        self.scan(&self.config.core.vault_path, &mut tasks)?;
        Ok(tasks)
    }

    fn scan(&self, path: &Path, tasks: &mut VaultData) -> Result<()> {
        if self.config.core.ignored.contains(&path.to_path_buf()) {
            debug!("Ignoring {path:?} (ignored list)");
            return Ok(());
        }

        for entry in path.read_dir()?.filter_map(Result::ok) {
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();
            let entry_path = entry.path();

            if !self.config.core.parse_dot_files && name.starts_with('.') {
                debug!("Ignoring {name:?} (dot file)");
                continue;
            }
            if self.config.core.ignored.contains(&entry_path) {
                debug!("Ignoring {name:?} (ignored list)");
                continue;
            }

            let VaultData::Directory(_, children) = tasks else {
                bail!("Error while scanning directories, FileEntry was not a Directory");
            };

            if entry.file_type()?.is_dir() {
                let mut new_child = VaultData::Directory(name.to_string(), vec![]);
                self.scan(&entry_path, &mut new_child)?;

                if let VaultData::Directory(_, c) = &new_child
                    && !c.is_empty()
                {
                    children.push(new_child);
                }
            } else {
                let ext = entry_path.extension().and_then(|s| s.to_str());
                if ext.is_none_or(|e| !e.eq_ignore_ascii_case("md")) {
                    debug!("Ignoring {name:?} (not a .md file)");
                    continue;
                }
                if let Some(file_tasks) = self.parse_file(&entry) {
                    children.push(file_tasks);
                }
            }
        }
        Ok(())
    }

    fn parse_file(&self, entry: &DirEntry) -> Option<VaultData> {
        debug!("Parsing {:?}", entry.file_name());
        let content = fs::read_to_string(entry.path()).unwrap_or_default();
        let mut parser = ParserFileEntry {
            config: &self.config,
            path: entry.path(),
        };

        parser.parse_file(&content.as_str())
    }
}
