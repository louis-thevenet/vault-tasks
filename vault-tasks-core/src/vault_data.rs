use std::{fmt::Display, path::PathBuf};

use super::{task::Task, tracker::Tracker};
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Vault {
        name: String,
        path: PathBuf,
        content: Vec<Node>,
    },
    Directory {
        name: String,
        path: PathBuf,
        content: Vec<Node>,
    },
    File {
        name: String,
        path: PathBuf,
        content: Vec<FileEntry>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileEntry {
    Header {
        name: String,
        heading_level: usize,
        content: Vec<FileEntry>,
    },
    Task(Task),
    Tracker(Tracker),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultData {
    pub root: Vec<Node>,
}

impl VaultData {
    pub fn new(root: Vec<Node>) -> Self {
        Self { root }
    }
    pub fn empty() -> Self {
        Self { root: vec![] }
    }
}

impl Display for VaultData {
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
            file_entry: &VaultData,
            f: &mut std::fmt::Formatter,
            depth: usize,
        ) -> std::fmt::Result {
            match file_entry {
                VaultData::Header(_, header, entries) => {
                    write_underline_with_indent(&header.to_string(), depth, f)?;
                    for entry in entries {
                        fmt_aux(entry, f, depth + 1)?;
                    }
                }
                VaultData::Vault {
                    short_name: name,
                    content: entries,
                    path: _,
                }
                | VaultData::Directory(name, entries) => {
                    write_underline_with_indent(&name.to_string(), depth, f)?;
                    for entry in entries {
                        fmt_aux(entry, f, depth + 1)?;
                    }
                }
                VaultData::Task(task) => {
                    for line in task.to_string().split('\n') {
                        write_indent(depth, f)?;
                        writeln!(f, "{line}")?;
                    }

                    for subtask in &task.subtasks {
                        for line in VaultData::Task(subtask.clone()).to_string().split('\n') {
                            write_indent(depth + 1, f)?;
                            writeln!(f, "{line}")?;
                        }
                    }
                }
                VaultData::Tracker(tracker) => {
                    for line in tracker.to_string().split('\n') {
                        write_indent(depth, f)?;
                        writeln!(f, "{line}")?;
                    }
                }
                VaultData::Root { vaults } => {
                    for vault in vaults {
                        fmt_aux(vault, f, depth + 1)?;
                    }
                }
            }
            Ok(())
        }
        fmt_aux(self, f, 0)
    }
}
