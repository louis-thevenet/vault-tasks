use color_eyre::Result;
use config::Config;
use file_entry::FileEntry;
use std::fmt::Display;
use tracing::error;
use vault_parser::VaultParser;

mod config;
pub mod file_entry;
mod parser;
mod task;
mod vault_parser;

pub struct TaskManager {
    tasks: Vec<FileEntry>,
}
impl TaskManager {
    pub fn load_from_config(config: &Config) -> Result<Self> {
        let vault_parser = VaultParser::new(config.clone());
        let tasks = vault_parser.scan_vault()?;
        Self::rewrite_vault_tasks(config, &tasks)?;
        // let tasks = vault_parser.scan_vault()?; // is not strictly necessary since tasks shouldn't change
        Ok(Self { tasks })
    }

    fn rewrite_vault_tasks(config: &Config, tasks: &Vec<FileEntry>) -> Result<()> {
        fn explore_tasks_rec(
            config: &Config,
            filename: &str,
            file_entry: &FileEntry,
        ) -> Result<()> {
            match file_entry {
                FileEntry::Header(_, children) => children
                    .iter()
                    .try_for_each(|c| explore_tasks_rec(config, filename, c))?,
                FileEntry::Task(task, subtasks) => {
                    task.fix_task_attributes(config, filename)?;
                    subtasks
                        .iter()
                        .try_for_each(|s| explore_tasks_rec(config, filename, s))?;
                }
            }
            Ok(())
        }
        for task in tasks {
            match task {
                FileEntry::Header(filename, content) => content
                    .iter()
                    .try_for_each(|c| explore_tasks_rec(config, filename, c))?,
                FileEntry::Task(_, _) => error!("FileEntry started with a Task:\n{task}"),
            }
        }
        Ok(())
    }
}
impl Display for TaskManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for task in &self.tasks {
            write!(f, "{task}")?;
        }
        Ok(())
    }
}
