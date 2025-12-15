use color_eyre::{Result, eyre::bail};

use std::{collections::HashSet, ffi::OsString, fmt::Display, iter::Peekable, path::Iter};
use vault_data::Vaults;

use filter::Filter;
use tracing::error;
use vault_parser::VaultParser;

use crate::{
    config::TasksConfig,
    task::Task,
    vault_data::{FileEntryNode, VaultNode},
};

pub mod config;
pub mod date;
pub mod filter;
pub mod logging;
pub mod parser;
pub mod sorter;
pub mod task;
pub mod tracker;
pub mod vault_data;
mod vault_parser;

// Re-export logging functions for easier access
pub use logging::init as init_logging;

pub struct TaskManager {
    pub tasks_refactored: Vaults,
    config: TasksConfig,
    pub tags: HashSet<String>,
    pub current_filter: Option<Filter>,
}
impl Default for TaskManager {
    fn default() -> Self {
        Self {
            tasks_refactored: Vaults { root: vec![] }, // TODO: will replace tasks eventually
            tags: HashSet::new(),
            current_filter: None,
            config: TasksConfig::default(),
        }
    }
}
// Helper enum to unify return types
#[derive(Clone, Debug)]
pub enum Found {
    Root(Vaults),
    Node(VaultNode),
    FileEntry(FileEntryNode),
}
impl TaskManager {
    /// Loads a vault from a `Config` and returns a `TaskManager`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the vault can't be loaded.
    pub fn load_from_config(config: &TasksConfig) -> Result<Self> {
        let mut res = Self::default();
        res.reload(config)?;
        Ok(res)
    }

    /// Reloads the `VaultData` from file system.
    ///
    /// # Errors
    ///
    /// This function will return an error if the vault can't be parsed, or if tasks can't be fixed (relative dates are replaced by fixed dates for example).
    pub fn reload(&mut self, config: &TasksConfig) -> Result<()> {
        self.config = config.clone();
        if self
            .config
            .core
            .vault_path
            .to_str()
            .is_some_and(str::is_empty)
        {
            bail!( "No vault path provided (use `--vault-path <PATH>`) and no default path set in config file".to_string(), );
        }
        if !self.config.core.vault_path.exists() && !cfg!(test) {
            bail!(
                "Vault path does not exist: {:?}",
                self.config.core.vault_path
            );
        }

        let vault_parser = VaultParser::new(config.clone());
        let tasks = vault_parser.scan_vault()?;

        self.tasks_refactored = tasks;

        Self::rewrite_vault_tasks(config, &self.tasks_refactored)
            .unwrap_or_else(|e| error!("Failed to fix tasks: {e}"));

        let mut tags = HashSet::new();
        Self::collect_tags(&self.tasks_refactored, &mut tags);
        self.tags = tags;
        Ok(())
    }
    /// Explores every `NewFileEntry` from the vault and applies the given function `f` on it.
    pub fn map_file_entries(
        tasks: &Vaults,
        f: &mut impl FnMut(&FileEntryNode) -> FileEntryNode,
    ) -> Vaults {
        fn explore_nodes(
            node: &VaultNode,
            f: &mut impl FnMut(&FileEntryNode) -> FileEntryNode,
        ) -> VaultNode {
            match node.clone() {
                VaultNode::Vault {
                    name,
                    path,
                    content,
                } => VaultNode::Vault {
                    name,
                    path,
                    content: content.iter().map(|v| explore_nodes(v, f)).collect(),
                },
                VaultNode::Directory {
                    name,
                    path,
                    content,
                } => VaultNode::Directory {
                    name,
                    path,
                    content: content.iter().map(|v| explore_nodes(v, f)).collect(),
                },
                VaultNode::File {
                    name,
                    path,
                    content,
                } => VaultNode::File {
                    name,
                    path,
                    content: content.iter().map(f).collect(),
                },
            }
        }
        let new_root = tasks.root.iter().map(|n| explore_nodes(n, f)).collect();
        Vaults { root: new_root }
    }

    /// Explores the vault and fills a `&mut HashSet<String>` with every tags found.
    pub fn collect_tags(tasks: &Vaults, tags: &mut HashSet<String>) {
        fn gather_tags_from_file_entry(file_entry: &FileEntryNode, tags: &mut HashSet<String>) {
            match file_entry {
                FileEntryNode::Task(task) => {
                    task.tags.clone().unwrap_or_default().iter().for_each(|t| {
                        tags.insert(t.clone());
                    });
                    task.subtasks.iter().for_each(|subtask| {
                        gather_tags_from_file_entry(&FileEntryNode::Task(subtask.clone()), tags);
                    });
                }
                FileEntryNode::Tracker(_tracker) => (),
                FileEntryNode::Header { content, .. } => {
                    for c in content {
                        gather_tags_from_file_entry(c, tags);
                    }
                }
            }
        }
        Self::map_file_entries(tasks, &mut |file_entry: &FileEntryNode| {
            gather_tags_from_file_entry(file_entry, tags);
            file_entry.clone()
        });
    }

    pub fn add_task(&mut self, task: &Task) -> Result<()> {
        task.fix_task_attributes(&self.config)?;
        self.reload(&self.config.clone())
    }
    /// Follows the `path` to retrieve the correct `VaultData`.
    ///
    /// # Errors
    /// Will return an error if the vault is empty or the path cannot be resolved
    pub fn resolve_path(&self, path: &[String]) -> Result<Found> {
        fn aux_node(node: &VaultNode, path: &[String], path_index: usize) -> Option<Found> {
            if path_index == path.len() {
                return Some(Found::Node(node.clone()));
            }

            match node {
                VaultNode::Vault { name, content, .. }
                | VaultNode::Directory { name, content, .. } => {
                    if *name == path[path_index] {
                        // Check if we're at the end of the path
                        if path_index + 1 == path.len() {
                            return Some(Found::Node(node.clone()));
                        }
                        // Otherwise, continue recursing into children
                        return content
                            .iter()
                            .find_map(|child| aux_node(child, path, path_index + 1));
                    }
                }
                VaultNode::File { name, content, .. } => {
                    if *name == path[path_index] {
                        // If we're at the end of the path, return file entries as nodes
                        if path_index + 1 == path.len() {
                            // Wrap each file entry in a temporary file node for conversion
                            return Some(Found::Node(node.clone()));
                        }
                        // Navigate into file entries
                        return content
                            .iter()
                            .find_map(|entry| aux_file_entry(entry, path, path_index + 1));
                    }
                }
            }
            None
        }

        fn aux_file_entry(
            file_entry: &FileEntryNode,
            path: &[String],
            path_index: usize,
        ) -> Option<Found> {
            if path_index == path.len() {
                return Some(Found::FileEntry(file_entry.clone()));
            }

            match file_entry {
                FileEntryNode::Header { name, content, .. } => {
                    if *name == path[path_index] {
                        // Check if we're at the end of the path
                        if path_index + 1 == path.len() {
                            return Some(Found::FileEntry(file_entry.clone()));
                        }
                        // Otherwise, continue recursing into children
                        return content
                            .iter()
                            .find_map(|child| aux_file_entry(child, path, path_index + 1));
                    }
                }
                FileEntryNode::Task(task) => {
                    if task.name == path[path_index] {
                        // Check if we're at the end of the path
                        if path_index + 1 == path.len() {
                            return Some(Found::FileEntry(file_entry.clone()));
                        }
                        // Otherwise, continue recursing into subtasks
                        let subtask_entries: Vec<FileEntryNode> = task
                            .subtasks
                            .iter()
                            .map(|t| FileEntryNode::Task(t.clone()))
                            .collect();
                        return subtask_entries
                            .iter()
                            .find_map(|st| aux_file_entry(st, path, path_index + 1));
                    }
                }
                FileEntryNode::Tracker(tracker) => {
                    if tracker.name == path[path_index] {
                        // Check if we're at the end of the path
                        if path_index + 1 == path.len() {
                            return Some(Found::FileEntry(file_entry.clone()));
                        }
                        // Trackers can't be entered, so return None
                        return None;
                    }
                }
            }
            None
        }
        // apply filter
        let Some(filtered_tasks) = filter::filter(&self.tasks_refactored, &self.current_filter)
        else {
            bail!("No tasks found after applying filter");
        };
        // If path is empty, return all root nodes
        if path.is_empty() {
            return Ok(Found::Root(filtered_tasks.clone()));
        }

        // Try to find entries in each vault root node
        for node in &filtered_tasks.root {
            if let Some(found) = aux_node(node, path, 0) {
                return Ok(found); // path only resolves to one node
            }
        }

        bail!("Couldn't find entries at path: {:?}", path)
    }

    /// Recursively calls `Task.fix_task_attributes` on every task from the vault.
    fn rewrite_vault_tasks(config: &TasksConfig, tasks: &Vaults) -> Result<()> {
        fn explore_node(node: &VaultNode, config: &TasksConfig) -> Result<()> {
            match node {
                VaultNode::Vault { content, .. } | VaultNode::Directory { content, .. } => {
                    for c in content {
                        explore_node(c, config)?;
                    }
                }
                VaultNode::File {
                    name: _, content, ..
                } => {
                    for file_entry in content {
                        match file_entry {
                            FileEntryNode::Header { content, .. } => {
                                for c in content {
                                    explore_file_entry(c, config)?;
                                }
                            }
                            _ => {
                                explore_file_entry(file_entry, config)?;
                            }
                        }
                    }
                }
            }
            Ok(())
        }
        fn explore_file_entry(file_entry: &FileEntryNode, config: &TasksConfig) -> Result<()> {
            match file_entry {
                FileEntryNode::Header { content, .. } => {
                    for c in content {
                        explore_file_entry(c, config)?;
                    }
                }
                FileEntryNode::Task(task) => {
                    task.fix_task_attributes(config)?;
                    for t in &task.subtasks {
                        t.fix_task_attributes(config)?;
                    }
                }
                FileEntryNode::Tracker(tracker) => {
                    tracker.fix_tracker_attributes(config)?;
                }
            }
            Ok(())
        }

        tasks
            .root
            .iter()
            .try_for_each(|node| explore_node(node, config))?;
        Ok(())
    }

    /// Whether the path resolves to something that can be entered or not.
    /// Directories, Headers and Tasks with subtasks can be entered.
    #[must_use]
    pub fn can_enter(&mut self, selected_header_path: &[String]) -> bool {
        fn aux_node(node: &VaultNode, selected_header_path: &[String], path_index: usize) -> bool {
            if path_index == selected_header_path.len() {
                true
            } else {
                match node {
                    VaultNode::Vault { name, content, .. }
                    | VaultNode::Directory { name, content, .. } => {
                        if *name == selected_header_path[path_index] {
                            return content
                                .iter()
                                .any(|c| aux_node(c, selected_header_path, path_index + 1));
                        }
                    }
                    VaultNode::File { name, content, .. } => {
                        if *name == selected_header_path[path_index] {
                            return content
                                .iter()
                                .any(|c| aux_file_entry(c, selected_header_path, path_index + 1));
                        }
                    }
                }
                false
            }
        }

        fn aux_file_entry(
            file_entry: &FileEntryNode,
            selected_header_path: &[String],
            path_index: usize,
        ) -> bool {
            if path_index == selected_header_path.len() {
                true
            } else {
                match file_entry {
                    FileEntryNode::Header { name, content, .. } => {
                        if *name == selected_header_path[path_index] {
                            return content
                                .iter()
                                .any(|c| aux_file_entry(c, selected_header_path, path_index + 1));
                        }
                        false
                    }
                    FileEntryNode::Task(task) => {
                        if task.name == selected_header_path[path_index] {
                            return task.subtasks.iter().any(|t| {
                                aux_file_entry(
                                    &FileEntryNode::Task(t.clone()),
                                    selected_header_path,
                                    path_index + 1,
                                )
                            });
                        }
                        false
                    }
                    FileEntryNode::Tracker(_tracker) => false, // Trackers can't be entered at the moment
                                                               // I plan on giving access to its categories someday
                }
            }
        }

        self.tasks_refactored
            .root
            .iter()
            .any(|node| aux_node(node, selected_header_path, 0))
    }
}
impl Display for TaskManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.tasks_refactored)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::TaskManager;

    use crate::{
        Found,
        task::Task,
        vault_data::{FileEntryNode, VaultNode, Vaults},
    };

    #[test]
    fn test_get_vault_data() {
        use std::path::Path;

        // Create tasks that will be in a header
        let task1 = Task {
            name: "test task 1".to_string(),
            line_number: Some(8),
            description: Some("test\ndesc".to_string()),
            ..Default::default()
        };
        let task2 = Task {
            name: "test task 2".to_string(),
            line_number: Some(9),
            description: Some("another desc".to_string()),
            ..Default::default()
        };
        let task3 = Task {
            name: "test task 3".to_string(),
            line_number: Some(10),
            ..Default::default()
        };

        // Build a vault structure: Vault -> Directory -> File -> Headers -> Tasks
        let file_content = vec![
            FileEntryNode::Header {
                name: "1".to_string(),
                heading_level: 1,
                content: vec![FileEntryNode::Header {
                    name: "2".to_string(),
                    heading_level: 2,
                    content: vec![],
                }],
            },
            FileEntryNode::Header {
                name: "1.2".to_string(),
                heading_level: 1,
                content: vec![
                    FileEntryNode::Header {
                        name: "3".to_string(),
                        heading_level: 3,
                        content: vec![],
                    },
                    FileEntryNode::Header {
                        name: "4".to_string(),
                        heading_level: 2,
                        content: vec![
                            FileEntryNode::Task(task1.clone()),
                            FileEntryNode::Task(task2.clone()),
                            FileEntryNode::Task(task3.clone()),
                        ],
                    },
                ],
            },
        ];

        let test_file = VaultNode::File {
            name: "Test".to_string(),
            path: Path::new("test/Test.md").into(),
            content: file_content,
        };

        let test_directory = VaultNode::Directory {
            name: "test".to_string(),
            path: Path::new("test").into(),
            content: vec![test_file],
        };

        let vault_data = Vaults {
            root: vec![test_directory],
        };

        let task_mgr = TaskManager {
            tasks_refactored: vault_data,
            tags: HashSet::new(),
            ..Default::default()
        };

        // Test path to empty header "2" - should return the header FileEntry
        let path = vec![
            String::from("test"),
            String::from("Test"),
            String::from("1"),
            String::from("2"),
        ];
        let found = task_mgr.resolve_path(&path).unwrap();
        let expected_header = FileEntryNode::Header {
            name: "2".to_string(),
            heading_level: 2,
            content: vec![],
        };
        match found {
            Found::FileEntry(entry) => assert_eq!(expected_header, entry),
            _ => panic!("Expected FileEntry, got {found:?}"),
        }

        // Test path to header "4" with tasks
        let path = vec![
            String::from("test"),
            String::from("Test"),
            String::from("1.2"),
            String::from("4"),
        ];
        let found = task_mgr.resolve_path(&path).unwrap();
        let expected_header_with_tasks = FileEntryNode::Header {
            name: "4".to_string(),
            heading_level: 2,
            content: vec![
                FileEntryNode::Task(task1),
                FileEntryNode::Task(task2),
                FileEntryNode::Task(task3),
            ],
        };
        match found {
            Found::FileEntry(entry) => assert_eq!(expected_header_with_tasks, entry),
            _ => panic!("Expected FileEntry, got {found:?}"),
        }
    }
}
