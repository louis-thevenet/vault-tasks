use color_eyre::{eyre::bail, Result};
use std::{fmt::Display, path::PathBuf};
use vault_data::VaultData;

use tracing::{debug, error};
use vault_parser::VaultParser;

use crate::config::Config;

// mod config;
mod parser;
pub mod task;
pub mod vault_data;
mod vault_parser;

pub struct TaskManager {
    pub tasks: VaultData,
}
impl Default for TaskManager {
    fn default() -> Self {
        let config = Config::new().unwrap_or_default();
        Self::load_from_config(&config)
    }
}
impl TaskManager {
    pub fn load_from_config(config: &Config) -> Self {
        let vault_parser = VaultParser::new(config.clone());
        let tasks = vault_parser.scan_vault().unwrap_or_else(|e| {
            error!("Failed to scan vault: {e}");
            VaultData::Header("Empty vault, an error may have occured".to_owned(), vec![])
        });

        Self::rewrite_vault_tasks(config, &tasks)
            .unwrap_or_else(|e| error!("Failed to fix tasks' due dates: {e}"));

        debug!("\n{}", tasks);
        Self { tasks }
    }

    fn rewrite_vault_tasks(config: &Config, tasks: &VaultData) -> Result<()> {
        fn explore_tasks_rec(
            config: &Config,
            filename: &mut PathBuf,
            file_entry: &VaultData,
        ) -> Result<()> {
            match file_entry {
                VaultData::Header(_, children) => {
                    children
                        .iter()
                        .try_for_each(|c| explore_tasks_rec(config, filename, c))?;
                }
                VaultData::Task(task) => {
                    task.fix_task_attributes(config, filename)?;
                    task.subtasks
                        .iter()
                        .try_for_each(|t| t.fix_task_attributes(config, filename))?;
                }
                VaultData::Directory(dir_name, children) => {
                    let mut filename = filename.clone();
                    filename.push(dir_name);
                    children
                        .iter()
                        .try_for_each(|c| explore_tasks_rec(config, &mut filename.clone(), c))?;
                }
            }
            Ok(())
        }
        explore_tasks_rec(config, &mut PathBuf::new(), tasks)
    }

    pub fn get_entries(
        &self,
        selected_header_path: Vec<String>,
    ) -> Result<(Vec<String>, Vec<String>)> {
        fn aux(
            file_entry: Vec<VaultData>,
            selected_header_path: Vec<String>,
            path_index: usize,
        ) -> Result<(Vec<String>, Vec<String>)> {
            if path_index == selected_header_path.len() {
                let mut res = vec![];
                let mut prefixes = vec![];
                for entry in file_entry {
                    match entry {
                        VaultData::Directory(name, _) => {
                            res.push(name.clone());
                            prefixes.push(if name.contains(".md") {
                                "📄".to_owned()
                            } else {
                                "📁".to_owned()
                            });
                        }
                        VaultData::Header(name, _) => {
                            res.push(name);
                            prefixes.push("🖊️".to_owned());
                        }
                        VaultData::Task(_) => todo!(),
                    }
                }
                Ok((prefixes, res))
            } else {
                for entry in file_entry {
                    match entry {
                        VaultData::Directory(name, children)
                        | VaultData::Header(name, children) => {
                            if name == selected_header_path[path_index] {
                                return aux(children, selected_header_path, path_index + 1);
                            }
                        }
                        VaultData::Task(_) => todo!(),
                    }
                }
                bail!("Couldn't find corresponding entry");
            }
        }

        let VaultData::Directory(_, entries) = self.tasks.clone() else {
            bail!("First layer of VaultData was not a Directory")
        };
        aux(entries, selected_header_path, 0)
    }
}
impl Display for TaskManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.tasks)?;
        Ok(())
    }
}
