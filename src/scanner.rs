use crate::{
    config::Config,
    core::{Task, TaskState},
};
use anyhow::{bail, Result};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use log::{debug, info};
use std::{
    collections::HashSet,
    fs::{self, DirEntry},
    path::Path,
};

pub struct Scanner {
    config: Config,
}

impl Scanner {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
    pub fn scan_vault(self) -> Result<HashSet<Task>> {
        let mut set = HashSet::<Task>::new();
        info!("Scanning {:?}", self.config.vault_path);
        self.scan(&self.config.vault_path, &mut set)?;
        Ok(set)
    }

    fn scan(&self, path: &Path, set: &mut HashSet<Task>) -> Result<()> {
        let entries = path.read_dir()?;

        debug!("Opening directory {:?}", path);
        for entry_err in entries {
            if let Ok(entry) = entry_err {
                let name = entry.file_name().into_string().unwrap();
                if name.starts_with(".") {
                    debug!("Skipping {name}");
                    continue;
                }
                if entry.path().is_dir() {
                    // recursive call for this subdir
                    self.scan(&entry.path(), set)?;
                } else {
                    if let Some(tasks) = Self::scan_file(entry) {
                        info!("Tasks found in {name} : {:#?}", tasks);
                        for task in tasks {
                            set.insert(task);
                        }
                    };
                }
            } else {
                bail!("Error while reading an entry from {:?}", path)
            }
        }

        Ok(())
    }

    fn scan_file(entry: DirEntry) -> Option<Vec<Task>> {
        let content = fs::read_to_string(entry.path()).unwrap_or_default();
        let mut tasks = vec![];
        for mut line in content.split('\n') {
            let to_do = line.starts_with("- [ ]");
            let done = line.starts_with("- [X]");
            if !(to_do || done) {
                continue;
            }

            line = line
                .strip_prefix(if to_do { "- [ ] " } else { "- [X] " })
                .unwrap();
            tasks.push(Task {
                name: line.to_string(),
                ..Default::default()
            });
        }
        if tasks.is_empty() {
            None
        } else {
            Some(tasks)
        }
    }
}
