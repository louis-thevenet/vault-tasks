use color_eyre::Result;
use rayon::prelude::*;
use std::{fs, path::Path};
use tracing::{debug, info};

use crate::{
    TasksConfig,
    parser::parser_file_entry::ParserFileEntry,
    vault_data::{FileEntryNode, VaultNode, Vaults},
};

pub struct VaultParser {
    config: TasksConfig,
}

impl VaultParser {
    pub const fn new(config: TasksConfig) -> Self {
        Self { config }
    }
    pub fn scan_vault(&self) -> Result<Vaults> {
        // TODO : multiple vaults here
        let tasks = VaultNode::Vault {
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
            content: self.scan_directory(&self.config.core.vault_path)?,
        };
        info!("Scanning {:?}", self.config.core.vault_path);
        Ok(Vaults { root: vec![tasks] })
    }

    fn scan_directory(&self, path: &Path) -> Result<Vec<VaultNode>> {
        if self.config.core.ignored.contains(&path.to_path_buf()) {
            debug!("Ignoring {path:?} (ignored list)");
            return Ok(vec![]);
        }

        let entries: Vec<_> = path.read_dir()?.filter_map(Result::ok).collect();

        entries
            .into_par_iter()
            .map(|entry| self.scan_entry(&entry))
            .collect::<Result<Vec<_>>>()
            .map(|nodes| nodes.into_iter().flatten().collect())
    }

    fn scan_entry(&self, entry: &std::fs::DirEntry) -> Result<Option<VaultNode>> {
        let entry_path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();

        if !self.config.core.parse_dot_files && name.starts_with('.') {
            debug!("Ignoring {name:?} (dot file)");
            return Ok(None);
        }
        if self.config.core.ignored.contains(&entry_path) {
            debug!("Ignoring {name:?} (ignored list)");
            return Ok(None);
        }

        if entry.file_type()?.is_dir() {
            let content = self.scan_directory(&entry_path)?;
            if content.is_empty() {
                return Ok(None);
            }
            return Ok(Some(VaultNode::Directory {
                name: name.to_string(),
                content,
                path: entry_path,
            }));
        }

        let ext = entry_path.extension().and_then(|s| s.to_str());
        if ext.is_none_or(|e| !e.eq_ignore_ascii_case("md")) {
            debug!("Ignoring {name:?} (not a .md file)");
            return Ok(None);
        }

        let inner_content = self.parse_file(&entry_path);
        if inner_content.is_empty() {
            return Ok(None);
        }

        Ok(Some(VaultNode::File {
            name: name.to_string(),
            path: entry_path,
            content: inner_content,
        }))
    }

    fn parse_file(&self, path: &Path) -> Vec<FileEntryNode> {
        debug!("Parsing {:?}", path.file_name());
        let content = fs::read_to_string(path).unwrap_or_default();
        let mut parser = ParserFileEntry {
            config: &self.config,
            path: path.to_path_buf(),
        };

        parser.parse_file(&content.as_str())
    }
}
