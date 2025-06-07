use color_eyre::{Result, eyre::bail};
use std::{
    fs::{self, DirEntry},
    path::Path,
};
use tracing::{debug, info};

use crate::core::{TasksConfig, parser::parser_file_entry::ParserFileEntry};

use super::{task::Task, vault_data::VaultData};

pub struct VaultParser {
    config: TasksConfig,
}

impl VaultParser {
    pub const fn new(config: TasksConfig) -> Self {
        Self { config }
    }
    pub fn scan_vault(&self) -> Result<VaultData> {
        let mut tasks =
            VaultData::Directory(self.config.vault_path.to_str().unwrap().to_owned(), vec![]);
        info!("Scanning {:?}", self.config.vault_path);
        self.scan(&self.config.vault_path, &mut tasks)?;
        Ok(tasks)
    }
    pub fn parse_single_task(&self, task: &str, filename: &str) -> Result<Task> {
        let mut parser = ParserFileEntry {
            config: &self.config,
            filename: filename.to_string(),
        };
        debug!("{task}");
        match parser.parse_file(filename, &task) {
            Some(VaultData::Task(_)) => {
                bail!(
                    "Got a Task from {task}, should have been a Header then the Task, but this should never happen"
                )
            }

            Some(VaultData::Header(_, _, content)) => {
                // Files are always parsed as Headers
                if content.len() != 1 {
                    bail!("Expected single task in header, got: {content:?}");
                } else if let Some(VaultData::Task(t)) = content.first() {
                    let res = Task {
                        line_number: None, // Explicitly set to None, as it's not from a file
                        ..t.clone()
                    };
                    Ok(res)
                } else {
                    bail!("Expected a single Task in Header, got: {content:?}");
                }
            }
            Some(VaultData::Directory(_, _)) => bail!(
                "Got a Directory from {task}, should have been a Header then the Task, but this should never happen"
            ),
            _ => bail!("Task is malformed: `{task}`"),
        }
    }

    fn scan(&self, path: &Path, tasks: &mut VaultData) -> Result<()> {
        if self.config.ignored.contains(&path.to_owned()) {
            debug!("Ignoring {path:?} (ignored list)");
            return Ok(());
        }

        let entries = if path.is_dir() {
            path.read_dir()?
                .collect::<Vec<Result<DirEntry, std::io::Error>>>()
        } else {
            path.parent()
                .unwrap()
                .read_dir()?
                .filter(|e| {
                    let e = e.as_ref().unwrap();
                    e.file_name().eq(&path.file_name().unwrap())
                })
                .collect::<Vec<Result<DirEntry, std::io::Error>>>()
        };

        for entry_err in entries {
            let Ok(entry) = entry_err else { continue };
            let name = entry.file_name().into_string().unwrap();
            if !self.config.parse_dot_files && name.starts_with('.') {
                debug!("Ignoring {name:?} (dot file)");
                continue;
            }
            if self.config.ignored.contains(&entry.path()) {
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
                        if !c.is_empty() {
                            children.push(new_child);
                        }
                    }
                } else if !std::path::Path::new(
                    &entry.file_name().into_string().unwrap_or_default(),
                )
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
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
        let mut parser = ParserFileEntry {
            config: &self.config,
            filename: String::new(),
        };

        parser.parse_file(entry.file_name().to_str().unwrap(), &content.as_str())
    }
}
