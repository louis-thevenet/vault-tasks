use crate::{config::Config, scanner::Scanner, task::Task};
use anyhow::Result;
use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileEntry {
    // Header, Content
    Header(String, Vec<FileEntry>),
    // Task, Subtasks
    Task(Task, Vec<FileEntry>), // Should not contain headers
}

impl Display for FileEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn write_indent(indent_length: usize, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            (1..=indent_length).try_for_each(|_| write!(f, "\t"))?;
            Ok(())
        }
        fn write_underline_with_indent(
            text: &str,
            indent_length: usize,
            f: &mut std::fmt::Formatter,
        ) -> std::fmt::Result {
            write_indent(indent_length, f)?;
            writeln!(f, "{text}")?;
            write_indent(indent_length, f)?;
            for _i in 0..(text.len()) {
                write!(f, "‾")?;
            }
            writeln!(f)?;
            Ok(())
        }
        fn fmt_aux(
            file_entry: &FileEntry,
            f: &mut std::fmt::Formatter,
            depth: usize,
        ) -> std::fmt::Result {
            match file_entry {
                FileEntry::Header(header, entries) => {
                    write_underline_with_indent(&format!("{depth}. {header}"), depth, f)?;
                    for entry in entries {
                        fmt_aux(entry, f, depth + 1)?;
                    }
                }
                FileEntry::Task(task, subtasks) => {
                    for line in task.to_string().split('\n') {
                        write_indent(depth, f)?;
                        writeln!(f, "{line}")?;
                    }

                    for subtask in subtasks {
                        for line in subtask.to_string().split('\n') {
                            write_indent(depth + 1, f)?;
                            writeln!(f, "{line}")?;
                        }
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
