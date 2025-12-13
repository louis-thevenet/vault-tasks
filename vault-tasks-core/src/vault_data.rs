use std::{fmt::Display, path::PathBuf};

use format_utils::{write_indent, write_underline_with_indent};

use super::{task::Task, tracker::Tracker};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VaultData {
    /// Name, Content
    Directory(String, Vec<VaultData>),
    /// Name, Content
    Header(usize, String, Vec<VaultData>),
    /// Task, Subtasks
    Task(Task),
    /// Tracker
    Tracker(Tracker),
}

impl Display for VaultData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
                VaultData::Directory(name, entries) => {
                    write_underline_with_indent(&name.to_string(), depth, f)?;
                    for entry in entries {
                        fmt_aux(entry, f, depth + 1)?;
                    }
                }
                VaultData::Task(task) => {
                    for line in task.to_string().replace('\r', "").split('\n') {
                        write_indent(depth, f)?;
                        writeln!(f, "{line}")?;
                    }

                    for subtask in &task.subtasks {
                        for line in VaultData::Task(subtask.clone())
                            .to_string()
                            .replace('\r', "")
                            .split('\n')
                        {
                            write_indent(depth + 1, f)?;
                            writeln!(f, "{line}")?;
                        }
                    }
                }
                VaultData::Tracker(tracker) => {
                    for line in tracker.to_string().replace('\r', "").split('\n') {
                        write_indent(depth, f)?;
                        writeln!(f, "{line}")?;
                    }
                }
            }
            Ok(())
        }
        fmt_aux(self, f, 0)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
/// A node in the vault data structure, representing either a vault, directory, or file.
pub enum NewNode {
    Vault {
        name: String,
        path: PathBuf,
        content: Vec<NewNode>,
    },
    Directory {
        name: String,
        path: PathBuf,
        content: Vec<NewNode>,
    },
    File {
        name: String,
        path: PathBuf,
        content: Vec<NewFileEntry>,
    },
}
impl Display for NewNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn fmt_node_rec(
            node: &NewNode,
            f: &mut std::fmt::Formatter,
            depth: usize,
        ) -> std::fmt::Result {
            match node {
                NewNode::Vault { name, content, .. } | NewNode::Directory { name, content, .. } => {
                    write_underline_with_indent(&name.to_string(), depth, f)?;
                    for entry in content {
                        fmt_node_rec(entry, f, depth + 1)?;
                    }
                }
                NewNode::File { name, content, .. } => {
                    write_underline_with_indent(&name.to_string(), depth, f)?;
                    for entry in content {
                        for line in entry.to_string().split('\n') {
                            write_indent(depth + 1, f)?;
                            writeln!(f, "{line}")?;
                        }
                    }
                }
            }
            Ok(())
        }
        fmt_node_rec(self, f, 0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// An entry in a file, representing either a header, task, or tracker.
pub enum NewFileEntry {
    Header {
        name: String,
        heading_level: usize,
        content: Vec<NewFileEntry>,
    },
    Task(Task),
    Tracker(Tracker),
}
impl Display for NewFileEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn fmt_file_entry_rec(
            file_entry: &NewFileEntry,
            f: &mut std::fmt::Formatter,
            depth: usize,
        ) -> std::fmt::Result {
            match file_entry {
                NewFileEntry::Header {
                    name,
                    heading_level: _,
                    content,
                } => {
                    write_underline_with_indent(name, depth, f)?;
                    for entry in content {
                        fmt_file_entry_rec(entry, f, depth + 1)?;
                    }
                }
                NewFileEntry::Task(task) => {
                    for line in task.to_string().replace('\r', "").split('\n') {
                        write_indent(depth, f)?;
                        writeln!(f, "{line}")?;
                    }
                    for subtask in &task.subtasks {
                        for line in (NewFileEntry::Task(subtask.clone()))
                            .to_string()
                            .replace('\r', "") // redundant
                            .split('\n')
                        {
                            write_indent(depth + 1, f)?;
                            writeln!(f, "{line}")?;
                        }
                    }
                }
                NewFileEntry::Tracker(tracker) => {
                    for line in tracker.to_string().replace('\r', "").split('\n') {
                        write_indent(depth, f)?;
                        writeln!(f, "{line}")?;
                    }
                }
            }
            Ok(())
        }
        fmt_file_entry_rec(self, f, 0)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewVaultData {
    pub root: Vec<NewNode>,
}

impl NewVaultData {
    #[must_use]
    pub fn new(root: Vec<NewNode>) -> Self {
        Self { root }
    }
    #[must_use]
    pub fn empty() -> Self {
        Self { root: vec![] }
    }
}

impl Display for NewVaultData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for vault in &self.root {
            writeln!(f, "{vault}")?;
        }
        Ok(())
    }
}
mod format_utils {
    pub fn write_indent(indent_length: usize, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        (1..=indent_length).try_for_each(|_| write!(f, "\t"))?;
        Ok(())
    }
    pub fn write_underline_with_indent(
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
}
