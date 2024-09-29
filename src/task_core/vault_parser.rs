use color_eyre::{eyre::bail, Result};
use std::{
    fs::{self, DirEntry},
    path::Path,
};
use tracing::{debug, info};

use crate::{config::Config, task_core::parser::parser_file_entry::ParserFileEntry};

use super::vault_data::VaultData;

pub struct VaultParser {
    config: Config,
}

impl VaultParser {
    pub const fn new(config: Config) -> Self {
        Self { config }
    }
    pub fn scan_vault(&self) -> Result<VaultData> {
        let mut tasks = VaultData::Directory(
            self.config
                .tasks_config
                .vault_path
                .to_str()
                .unwrap()
                .to_owned(),
            vec![],
        );
        info!("Scanning {:?}", self.config.tasks_config.vault_path);
        self.scan(&self.config.tasks_config.vault_path, &mut tasks)?;
        Ok(tasks)
    }

    fn scan(&self, path: &Path, tasks: &mut VaultData) -> Result<()> {
        if self.config.tasks_config.ignored.contains(&path.to_owned()) {
            debug!("Ignoring {path:?} (ignored list)");
            return Ok(());
        }

        let entries = path.read_dir()?;
        for entry_err in entries {
            let Ok(entry) = entry_err else { continue };
            let name = entry.file_name().into_string().unwrap();
            if !self.config.tasks_config.parse_dot_files && name.starts_with('.') {
                debug!("Ignoring {name:?} (dot file)");
                continue;
            }
            if self.config.tasks_config.ignored.contains(&entry.path()) {
                debug!("Ignoring {name:?} (ignored list)");
                continue;
            }

            if let VaultData::Directory(_, children) = tasks {
                if entry.path().is_dir() {
                    // recursive call for this subdir
                    let mut new_child = VaultData::Directory(
                        entry.file_name().to_str().unwrap().to_owned(),
                        vec![],
                    );

                    self.scan(&entry.path(), &mut new_child)?;

                    if let VaultData::Directory(_, c) = new_child.clone() {
                        if c.is_empty() {
                        } else {
                            children.push(new_child);
                        }
                    }
                } else if !std::path::Path::new(
                    &entry.file_name().into_string().unwrap_or_default(),
                )
                .extension()
                .map_or(false, |ext| ext.eq_ignore_ascii_case("md"))
                {
                    debug!("Ignoring {name:?} (not a .md file)");
                    continue;
                } else if let Some(file_tasks) = self.parse_file(&entry) {
                    children.push(file_tasks);
                }
            } else {
                bail!("Error while scanning directories, FileEntry was not a Directory");
            }
        }
        Ok(())
    }

    fn parse_file(&self, entry: &DirEntry) -> Option<VaultData> {
        debug!("Parsing {:?}", entry.file_name());
        let content = fs::read_to_string(entry.path()).unwrap_or_default();
        let parser = ParserFileEntry {
            config: &self.config,
        };

        parser.parse_file(entry.file_name().to_str().unwrap(), &content.as_str())
    }
}
