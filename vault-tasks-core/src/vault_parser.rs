use color_eyre::{Result, eyre::bail};
use std::{
    fs::{self, DirEntry},
    path::Path,
};
use tracing::{debug, info};

use crate::{
    TasksConfig,
    parser::parser_file_entry::ParserFileEntry,
    vault_data::{NewFileEntry, NewNode, NewVaultData},
};

pub struct VaultParser {
    config: TasksConfig,
}

impl VaultParser {
    pub const fn new(config: TasksConfig) -> Self {
        Self { config }
    }
    pub fn scan_vault(&self) -> Result<NewVaultData> {
        // TODO : multiple vaults here
        let mut tasks = NewNode::Vault {
            name: self
                .config
                .core
                .vault_path
                .clone()
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("Vault")
                .to_string(),
            path: self.config.core.vault_path.clone(),
            content: vec![],
        };
        info!("Scanning {:?}", self.config.core.vault_path);
        self.scan(&self.config.core.vault_path, &mut tasks)?;
        Ok(NewVaultData { root: vec![tasks] })
    }

    fn scan(&self, path: &Path, tasks: &mut NewNode) -> Result<()> {
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

            let content = match tasks {
                NewNode::Vault { content, .. } | NewNode::Directory { content, .. } => content,
                NewNode::File { .. } => bail!(
                    "Error while scanning directories, FileEntry was not a Directory or Vault"
                ),
            };

            if entry.file_type()?.is_dir() {
                let new_content: Vec<NewNode> = vec![];
                let mut new_child = NewNode::Directory {
                    name: name.to_string(),
                    content: new_content,
                    path: entry_path.clone(),
                };
                self.scan(&entry_path, &mut new_child)?;

                // Check if the directory has content
                let has_content = match &new_child {
                    NewNode::Directory { content, .. } => !content.is_empty(),
                    _ => false,
                };
                if has_content {
                    content.push(new_child);
                }
            } else {
                let ext = entry_path.extension().and_then(|s| s.to_str());
                if ext.is_none_or(|e| !e.eq_ignore_ascii_case("md")) {
                    debug!("Ignoring {name:?} (not a .md file)");
                    continue;
                }
                if let Some(file_tasks) = self.parse_file(&entry) {
                    content.push(NewNode::File {
                        name: name.to_string(),
                        path: entry_path.clone(),
                        content: vec![file_tasks],
                    });
                }
            }
        }
        Ok(())
    }

    fn parse_file(&self, entry: &DirEntry) -> Option<NewFileEntry> {
        debug!("Parsing {:?}", entry.file_name());
        let content = fs::read_to_string(entry.path()).unwrap_or_default();
        let mut parser = ParserFileEntry {
            config: &self.config,
            path: entry.path(),
        };

        parser.parse_file(&content.as_str())
    }
}
