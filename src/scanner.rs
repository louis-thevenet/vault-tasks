use crate::{config::Config, core::Task, parser::parser_task::parse_task};
use anyhow::{bail, Result};
use log::error;
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
        for entry_err in entries {
            if let Ok(entry) = entry_err {
                let name = entry.file_name().into_string().unwrap();
                if name.starts_with(".") {
                    continue;
                }
                if entry.path().is_dir() {
                    // recursive call for this subdir
                    self.scan(&entry.path(), set)?;
                } else if let Some(tasks) = self.parse_file(entry) {
                    debug!("Tasks found in {name}");
                    for task in tasks {
                        debug!("\n{task}");
                        set.insert(task);
                    }
                }
            } else {
                bail!("Error while reading an entry from {:?}", path)
            }
        }

        Ok(())
    }

    fn parse_file(&self, entry: DirEntry) -> Option<Vec<Task>> {
        let content = fs::read_to_string(entry.path()).unwrap_or_default();
        let mut tasks = vec![];
        let mut lines = content.split('\n').peekable();
        while let Some(mut line) = lines.next() {
            let is_task = line.starts_with("- [") && line.chars().nth(4).unwrap_or(' ') == ']';
            if !is_task {
                continue;
            }
            let mut line_copy_to_parse = line;

            let mut has_desc = if let Some(next_line) = lines.peek() {
                next_line.starts_with(' ')
            } else {
                false
            };

            let description = if has_desc {
                let mut description_lines = vec![];
                while has_desc {
                    line = lines.next().unwrap(); // we know it exists
                    description_lines.push(line.trim_start()); // trim is not optimal, doesn't preserve eventual indenting
                    has_desc = lines.peek().map_or(false, |l| l.starts_with(' '));
                }
                Some(description_lines.join("\n"))
            } else {
                None
            };
            match parse_task(&mut line_copy_to_parse, description, &self.config) {
                Ok(task) => tasks.push(task),

                Err(error) => error!("Failed to parse \"{line_copy_to_parse}\"\n {error}"),
            };
        }

        if tasks.is_empty() {
            None
        } else {
            Some(tasks)
        }
    }
}
