use std::{fmt::Display, path::PathBuf};

use format_utils::{write_indent, write_underline_with_indent};

use super::task::Task;

#[derive(Debug, Clone, PartialEq, Eq)]
/// A node in the vault data structure, representing either a vault, directory, or file.
pub enum VaultNode {
    Vault {
        name: String,
        path: PathBuf,
        content: Vec<VaultNode>,
    },
    Directory {
        name: String,
        path: PathBuf,
        content: Vec<VaultNode>,
    },
    File {
        name: String,
        path: PathBuf,
        content: Vec<FileEntryNode>,
    },
}
impl Display for VaultNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn fmt_node_rec(
            node: &VaultNode,
            f: &mut std::fmt::Formatter,
            depth: usize,
        ) -> std::fmt::Result {
            match node {
                VaultNode::Vault { name, content, .. }
                | VaultNode::Directory { name, content, .. } => {
                    write_underline_with_indent(&name.to_string(), depth, f)?;
                    for entry in content {
                        fmt_node_rec(entry, f, depth + 1)?;
                    }
                }
                VaultNode::File { name, content, .. } => {
                    write_underline_with_indent(&name.to_string(), depth, f)?;
                    for entry in content {
                        // Use NewFileEntry's fmt_with_depth to format with proper indentation
                        entry.fmt_with_depth(f, depth + 1)?;
                    }
                }
            }
            Ok(())
        }
        fmt_node_rec(self, f, 0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// An entry in a file, representing either a header or task.
pub enum FileEntryNode {
    Header {
        name: String,
        heading_level: usize,
        content: Vec<FileEntryNode>,
    },
    Task(Task),
}

impl FileEntryNode {
    pub(crate) fn fmt_with_depth(
        &self,
        f: &mut std::fmt::Formatter,
        depth: usize,
    ) -> std::fmt::Result {
        match self {
            FileEntryNode::Header {
                name,
                heading_level: _,
                content,
            } => {
                write_underline_with_indent(name, depth, f)?;
                for entry in content {
                    entry.fmt_with_depth(f, depth + 1)?;
                }
            }
            FileEntryNode::Task(task) => {
                for line in task.to_string().replace('\r', "").split('\n') {
                    write_indent(depth, f)?;
                    writeln!(f, "{line}")?;
                }
                for subtask in &task.subtasks {
                    for line in (FileEntryNode::Task(subtask.clone()))
                        .to_string()
                        .split('\n')
                    {
                        write_indent(depth + 1, f)?;
                        writeln!(f, "{line}")?;
                    }
                }
            }
        }
        Ok(())
    }
}

impl Display for FileEntryNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.fmt_with_depth(f, 0)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vaults {
    pub root: Vec<VaultNode>,
}

impl Vaults {
    #[must_use]
    pub fn new(root: Vec<VaultNode>) -> Self {
        Self { root }
    }
    #[must_use]
    pub fn empty() -> Self {
        Self { root: vec![] }
    }
}

impl Display for Vaults {
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
