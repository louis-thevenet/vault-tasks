use color_eyre::{Result, eyre::bail};

use std::{
    collections::HashSet,
    fmt::Display,
    path::PathBuf,
};
use vault_data::{NewVaultData, VaultData};

use filter::Filter;
use tracing::error;
use vault_parser::VaultParser;

use crate::{
    config::TasksConfig,
    vault_data::{NewFileEntry, NewNode},
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

/// Temporary module for refactoring `VaultData` to the new structure.
/// Contains conversion functions and tests. Will be removed after refactoring is complete.
pub mod tmp_refactor;

// Re-export logging functions for easier access
pub use logging::init as init_logging;

pub struct TaskManager {
    pub tasks: VaultData,
    pub tasks_refactored: NewVaultData,
    config: TasksConfig,
    pub tags: HashSet<String>,
    pub current_filter: Option<Filter>,
}
impl Default for TaskManager {
    fn default() -> Self {
        Self {
            tasks: VaultData::Directory("Empty Vault".to_owned(), vec![]),
            tasks_refactored: NewVaultData { root: vec![] }, // TODO: will replace tasks eventually
            tags: HashSet::new(),
            current_filter: None,
            config: TasksConfig::default(),
        }
    }
}
// Helper enum to unify return types
#[derive(Clone)]
pub enum Found {
    Root(NewVaultData),
    Node(NewNode),
    FileEntry(NewFileEntry),
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

        self.tasks = tasks;
        // TODO: until parsing is refactored
        self.tasks_refactored = tmp_refactor::convert_legacy_to_new(vec![self.tasks.clone()]);

        Self::rewrite_vault_tasks(config, &self.tasks_refactored)
            .unwrap_or_else(|e| error!("Failed to fix tasks: {e}"));

        let mut tags = HashSet::new();
        Self::collect_tags(&self.tasks_refactored, &mut tags);
        self.tags = tags;
        Ok(())
    }
    /// Explores every `NewFileEntry` from the vault and applies the given function `f` on it.
    pub fn map_file_entries(
        tasks: &NewVaultData,
        f: &mut impl FnMut(&NewFileEntry) -> NewFileEntry,
    ) -> NewVaultData {
        fn explore_nodes(
            node: &NewNode,
            f: &mut impl FnMut(&NewFileEntry) -> NewFileEntry,
        ) -> NewNode {
            match node.clone() {
                NewNode::Vault {
                    name,
                    path,
                    content,
                } => NewNode::Vault {
                    name,
                    path,
                    content: content.iter().map(|v| explore_nodes(v, f)).collect(),
                },
                NewNode::Directory {
                    name,
                    path,
                    content,
                } => NewNode::Directory {
                    name,
                    path,
                    content: content.iter().map(|v| explore_nodes(v, f)).collect(),
                },
                NewNode::File {
                    name,
                    path,
                    content,
                } => NewNode::File {
                    name,
                    path,
                    content: content.iter().map(f).collect(),
                },
            }
        }
        let new_root = tasks.root.iter().map(|n| explore_nodes(n, f)).collect();
        NewVaultData { root: new_root }
    }

    /// Explores the vault and fills a `&mut HashSet<String>` with every tags found.
    pub fn collect_tags(tasks: &NewVaultData, tags: &mut HashSet<String>) {
        fn gather_tags_from_file_entry(file_entry: &NewFileEntry, tags: &mut HashSet<String>) {
            match file_entry {
                NewFileEntry::Task(task) => {
                    task.tags.clone().unwrap_or_default().iter().for_each(|t| {
                        tags.insert(t.clone());
                    });
                    task.subtasks.iter().for_each(|subtask| {
                        gather_tags_from_file_entry(&NewFileEntry::Task(subtask.clone()), tags);
                    });
                }
                NewFileEntry::Tracker(_tracker) => (),
                NewFileEntry::Header { content, .. } => {
                    for c in content {
                        gather_tags_from_file_entry(c, tags);
                    }
                }
            }
        }
        Self::map_file_entries(tasks, &mut |file_entry: &NewFileEntry| {
            gather_tags_from_file_entry(file_entry, tags);
            file_entry.clone()
        });
    }

    /// Follows the `path` to retrieve the correct `VaultData`.
    ///
    /// # Errors
    /// Will return an error if the vault is empty or the path cannot be resolved
    pub fn resolve_path(&self, path: &[String]) -> Result<Found> {
        fn aux_node(node: &NewNode, path: &[String], path_index: usize) -> Option<Found> {
            if path_index == path.len() {
                return Some(Found::Node(node.clone()));
            }

            match node {
                NewNode::Vault { name, content, .. } | NewNode::Directory { name, content, .. } => {
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
                NewNode::File { name, content, .. } => {
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
            file_entry: &NewFileEntry,
            path: &[String],
            path_index: usize,
        ) -> Option<Found> {
            if path_index == path.len() {
                return Some(Found::FileEntry(file_entry.clone()));
            }

            match file_entry {
                NewFileEntry::Header { name, content, .. } => {
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
                NewFileEntry::Task(task) => {
                    if task.name == path[path_index] {
                        // Check if we're at the end of the path
                        if path_index + 1 == path.len() {
                            return Some(Found::FileEntry(file_entry.clone()));
                        }
                        // Otherwise, continue recursing into subtasks
                        let subtask_entries: Vec<NewFileEntry> = task
                            .subtasks
                            .iter()
                            .map(|t| NewFileEntry::Task(t.clone()))
                            .collect();
                        return subtask_entries
                            .iter()
                            .find_map(|st| aux_file_entry(st, path, path_index + 1));
                    }
                }
                NewFileEntry::Tracker(tracker) => {
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

        // If path is empty, return all root nodes
        if path.is_empty() {
            return Ok(Found::Root(self.tasks_refactored.clone()));
        }

        // Try to find entries in each vault root node
        for node in &self.tasks_refactored.root {
            if let Some(found) = aux_node(node, path, 0) {
                return Ok(found); // path only resolves to one node
            }
        }

        bail!("Couldn't find entries at path: {:?}", path)
    }

    /// Recursively calls `Task.fix_task_attributes` on every task from the vault.
    fn rewrite_vault_tasks(config: &TasksConfig, tasks: &NewVaultData) -> Result<()> {
        fn explore_node(
            node: &NewNode,
            filename: &mut PathBuf,
            config: &TasksConfig,
        ) -> Result<()> {
            match node {
                NewNode::Vault { name, content, .. } | NewNode::Directory { name, content, .. } => {
                    let mut filename = filename.clone();
                    filename.push(name);
                    for c in content {
                        explore_node(c, &mut filename.clone(), config)?;
                    }
                }
                NewNode::File {
                    name: _, content, ..
                } => {
                    for file_entry in content {
                        match file_entry {
                            NewFileEntry::Header { content, .. } => {
                                for c in content {
                                    explore_file_entry(c, filename, config)?;
                                }
                            }
                            _ => {
                                explore_file_entry(file_entry, filename, config)?;
                            }
                        }
                    }
                }
            }
            Ok(())
        }
        fn explore_file_entry(
            file_entry: &NewFileEntry,
            filename: &mut PathBuf,
            config: &TasksConfig,
        ) -> Result<()> {
            match file_entry {
                NewFileEntry::Header { content, .. } => {
                    for c in content {
                        explore_file_entry(c, filename, config)?;
                    }
                }
                NewFileEntry::Task(task) => {
                    task.fix_task_attributes(config, filename)?;
                    for t in &task.subtasks {
                        t.fix_task_attributes(config, filename)?;
                    }
                }
                NewFileEntry::Tracker(tracker) => {
                    tracker.fix_tracker_attributes(config, filename)?;
                }
            }
            Ok(())
        }

        tasks
            .root
            .iter()
            .try_for_each(|node| explore_node(node, &mut PathBuf::new(), config))?;
        Ok(())
    }
    /// Retrieves the `VaultData` at the given `path`, and returns the entries to display.
    ///
    /// If the path ends with a task, the `task_preview_offset` parameter determines whether the function should return the task itself or its content (subtasks) as with directories and headers.
    pub fn get_vault_data_from_path(&self, path: &[String]) -> Result<VaultData> {
        /// Recursively searches for the entry in the vault nodes.
        /// `path_index` is the index of the current path element we are looking for.
        fn aux_node(
            node: &NewNode,
            selected_header_path: &[String],
            path_index: usize,
        ) -> Result<NewNode> {
            // Remaining path is empty?
            if path_index == selected_header_path.len() {
                Ok(node.clone())
            } else {
                match node {
                    NewNode::Vault { name, content, .. }
                    | NewNode::Directory { name, content, .. } => {
                        if *name == selected_header_path[path_index] {
                            if path_index + 1 == selected_header_path.len() {
                                return Ok(node.clone());
                            }
                            // Look for the child that matches the path
                            for child in content {
                                if let Ok(found) =
                                    aux_node(child, selected_header_path, path_index + 1)
                                {
                                    return Ok(found);
                                }
                            }
                        }
                        bail!("Couldn't find corresponding entry in vault/directory");
                    }
                    NewNode::File { name, content, .. } => {
                        if *name == selected_header_path[path_index] {
                            if path_index + 1 == selected_header_path.len() {
                                return Ok(node.clone());
                            }
                            // Look for the child that matches the path in file entries
                            for child in content {
                                if let Ok(found) =
                                    aux_file_entry(child, selected_header_path, path_index + 1)
                                {
                                    // Wrap the file entry in a temporary file node
                                    return Ok(NewNode::File {
                                        name: name.clone(),
                                        path: match node {
                                            NewNode::File { path, .. } => path.clone(),
                                            _ => unreachable!(),
                                        },
                                        content: vec![found],
                                    });
                                }
                            }
                        }
                        bail!("Couldn't find corresponding entry in file");
                    }
                }
            }
        }

        fn aux_file_entry(
            file_entry: &NewFileEntry,
            selected_header_path: &[String],
            path_index: usize,
        ) -> Result<NewFileEntry> {
            if path_index == selected_header_path.len() {
                Ok(file_entry.clone())
            } else {
                match file_entry {
                    NewFileEntry::Header { name, content, .. } => {
                        if *name == selected_header_path[path_index] {
                            if path_index + 1 == selected_header_path.len() {
                                return Ok(file_entry.clone());
                            }
                            for child in content {
                                if let Ok(found) =
                                    aux_file_entry(child, selected_header_path, path_index + 1)
                                {
                                    return Ok(found);
                                }
                            }
                        }
                        bail!("Couldn't find corresponding entry in header");
                    }
                    NewFileEntry::Task(task) => {
                        if task.name == selected_header_path[path_index] {
                            if path_index + 1 == selected_header_path.len() {
                                return Ok(file_entry.clone());
                            }
                            for child in &task.subtasks {
                                if let Ok(found) = aux_file_entry(
                                    &NewFileEntry::Task(child.clone()),
                                    selected_header_path,
                                    path_index + 1,
                                ) {
                                    return Ok(found);
                                }
                            }
                        }
                        bail!("Couldn't find corresponding entry in task");
                    }
                    NewFileEntry::Tracker(tracker) => {
                        if tracker.name == selected_header_path[path_index] {
                            if path_index + 1 == selected_header_path.len() {
                                return Ok(file_entry.clone());
                            }
                            bail!("Path was too long while we went down on a tracker")
                        }
                        bail!("Tracker name not matching")
                    }
                }
            }
        }

        // Try to find the entry in each vault root node
        for node in &self.tasks_refactored.root {
            if let Ok(found_node) = aux_node(node, path, 0) {
                // Convert the found node to legacy VaultData format
                let new_vault_data = NewVaultData {
                    root: vec![found_node],
                };
                let legacy = tmp_refactor::convert_new_to_legacy(&new_vault_data);
                if let Some(result) = legacy.first() {
                    return Ok(result.clone());
                }
            }
        }

        error!("Entry not found at path: {:?}", path);
        bail!("Entry not found at path")
    }

    /// Whether the path resolves to something that can be entered or not.
    /// Directories, Headers and Tasks with subtasks can be entered.
    #[must_use]
    pub fn can_enter(&mut self, selected_header_path: &[String]) -> bool {
        fn aux_node(node: &NewNode, selected_header_path: &[String], path_index: usize) -> bool {
            if path_index == selected_header_path.len() {
                true
            } else {
                match node {
                    NewNode::Vault { name, content, .. }
                    | NewNode::Directory { name, content, .. } => {
                        if *name == selected_header_path[path_index] {
                            return content
                                .iter()
                                .any(|c| aux_node(c, selected_header_path, path_index + 1));
                        }
                        true
                    }
                    NewNode::File { name, content, .. } => {
                        if *name == selected_header_path[path_index] {
                            return content
                                .iter()
                                .any(|c| aux_file_entry(c, selected_header_path, path_index + 1));
                        }
                        true
                    }
                }
            }
        }

        fn aux_file_entry(
            file_entry: &NewFileEntry,
            selected_header_path: &[String],
            path_index: usize,
        ) -> bool {
            if path_index == selected_header_path.len() {
                true
            } else {
                match file_entry {
                    NewFileEntry::Header { name, content, .. } => {
                        if *name == selected_header_path[path_index] {
                            return content
                                .iter()
                                .any(|c| aux_file_entry(c, selected_header_path, path_index + 1));
                        }
                        false
                    }
                    NewFileEntry::Task(task) => {
                        if task.name == selected_header_path[path_index] {
                            return task.subtasks.iter().any(|t| {
                                aux_file_entry(
                                    &NewFileEntry::Task(t.clone()),
                                    selected_header_path,
                                    path_index + 1,
                                )
                            });
                        }
                        false
                    }
                    NewFileEntry::Tracker(_tracker) => false, // Trackers can't be entered at the moment
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
        write!(f, "{}", self.tasks)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::TaskManager;

    use crate::{task::Task, tmp_refactor, vault_data::VaultData};

    #[test]
    fn test_get_vault_data() {
        let expected_tasks = vec![
            VaultData::Task(Task {
                name: "test".to_string(),
                line_number: Some(8),
                description: Some("test\ndesc".to_string()),
                ..Default::default()
            }),
            VaultData::Task(Task {
                name: "test".to_string(),
                line_number: Some(8),
                description: Some("test\ndesc".to_string()),
                ..Default::default()
            }),
            VaultData::Task(Task {
                name: "test".to_string(),
                line_number: Some(8),
                description: Some("test\ndesc".to_string()),
                ..Default::default()
            }),
        ];
        let expected_header = VaultData::Header(2, "2".to_string(), vec![]);
        let input = VaultData::Directory(
            "test".to_owned(),
            vec![VaultData::Header(
                0,
                "Test".to_string(),
                vec![
                    VaultData::Header(
                        1,
                        "1".to_string(),
                        vec![VaultData::Header(2, "2".to_string(), vec![])],
                    ),
                    VaultData::Header(
                        1,
                        "1.2".to_string(),
                        vec![
                            VaultData::Header(3, "3".to_string(), vec![]),
                            VaultData::Header(2, "4".to_string(), expected_tasks.clone()),
                        ],
                    ),
                ],
            )],
        );

        let refactored = tmp_refactor::convert_legacy_to_new(vec![input.clone()]);
        let task_mgr = TaskManager {
            tasks: input,
            tasks_refactored: refactored,
            tags: HashSet::new(),
            ..Default::default()
        };

        let path = vec![
            String::from("test"),
            String::from("Test"),
            String::from("1"),
            String::from("2"),
        ];
        let res = task_mgr.get_vault_data_from_path(&path).unwrap();
        assert_eq!(expected_header, res);

        let path = vec![
            String::from("test"),
            String::from("Test"),
            String::from("1.2"),
            String::from("4"),
        ];
        let res = task_mgr.get_vault_data_from_path(&path).unwrap();
        let expected_header = VaultData::Header(2, "4".to_string(), expected_tasks.clone());
        assert_eq!(expected_header, res);
    }
}
