use crate::{config::Config, scanner::Scanner, task::Task};
use anyhow::Result;
use std::fmt::Display;

#[derive(Debug)]
pub enum FileEntry {
    Header(String, Vec<FileEntry>),
    Tasks(Task),
}

impl Display for FileEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn fmt_aux(
            file_entry: &FileEntry,
            f: &mut std::fmt::Formatter,
            depth: usize,
        ) -> std::fmt::Result {
            match file_entry {
                FileEntry::Header(header, entries) => {
                    (0..depth).try_for_each(|_| write!(f, "\t"))?;
                    writeln!(f, "{header}")?;
                    for entry in entries {
                        fmt_aux(entry, f, depth + 1)?;
                    }
                }
                FileEntry::Tasks(task) => {
                    let task_str = task.to_string();
                    for line in task_str.split('\n') {
                        (0..=depth).try_for_each(|_| write!(f, "\t"))?;
                        writeln!(f, "{line}")?;
                    }
                }
            }
            Ok(())
        }
        fmt_aux(self, f, 0)
    }
}
pub struct TaskManager {
    tasks: Vec<FileEntry>,
}
impl TaskManager {
    pub fn load_from_config(config: Config) -> Result<Self> {
        let scanner = Scanner::new(config);
        let tasks = scanner.scan_vault()?;
        Ok(Self { tasks })
    }
}
