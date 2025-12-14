use color_eyre::{Result, eyre::bail};
use task::Task;

use std::{
    collections::HashSet,
    fmt::Display,
    path::{Path, PathBuf},
};
use vault_data::{NewVaultData, VaultData};

use filter::{Filter, filter};
use tracing::{debug, error};
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

    /// Follows the `selected_header_path` to retrieve the correct `VaultData`.
    /// Then returns every `VaultData` objects on the same layer.
    ///
    /// # Errors
    /// Will return an error if the vault is empty or the first layer is not a `VaultData::Directory`
    pub fn get_explorer_entries(&self, selected_header_path: &[String]) -> Result<Vec<VaultData>> {
        fn aux(
            file_entry: Vec<VaultData>,
            selected_header_path: &[String],
            path_index: usize,
        ) -> Result<Vec<VaultData>> {
            if path_index == selected_header_path.len() {
                Ok(file_entry)
            } else {
                for entry in file_entry {
                    match entry {
                        VaultData::Directory(name, children)
                        | VaultData::Header(_, name, children) => {
                            if name == selected_header_path[path_index] {
                                return aux(children, selected_header_path, path_index + 1);
                            }
                        }
                        VaultData::Task(task) => {
                            if task.name == selected_header_path[path_index] {
                                return aux(
                                    task.subtasks
                                        .iter()
                                        .map(|t| VaultData::Task(t.clone()))
                                        .collect(),
                                    selected_header_path,
                                    path_index + 1,
                                );
                            }
                        }
                        VaultData::Tracker(tracker) => {
                            bail!("Tried to list a Tracker's entries: {tracker}")
                        } // We can't enter Trackers so we won't ever have to resolve a path to one
                    }
                }
                bail!("Couldn't find corresponding entry");
            }
        }

        let filtered_tasks = filter(&self.tasks_refactored, &self.current_filter);
        match filtered_tasks {
            Some(VaultData::Directory(_, entries)) => aux(entries, selected_header_path, 0),
            None => bail!("Empty Vault"),
            _ => {
                error!("First layer of VaultData was not a Directory");
                bail!("First layer of VaultData was not a Directory")
            }
        }
    }

    /// Same as `get_explorer_entries`, but discards any children of the entries.
    ///
    /// # Errors
    ///
    /// This function will return an error if the path can't be resolved.
    pub fn get_explorer_entries_without_children(&self, path: &[String]) -> Result<Vec<VaultData>> {
        Ok(self
            .get_explorer_entries(path)? // Get the entries at the path
            .iter() // Discard every children
            .map(|vd| match vd {
                VaultData::Directory(name, _) => VaultData::Directory(name.clone(), vec![]),
                VaultData::Header(level, name, _) => {
                    VaultData::Header(*level, name.clone(), vec![])
                }
                VaultData::Task(t) => {
                    let mut t = t.clone();
                    t.subtasks = vec![]; // Discard subtasks
                    VaultData::Task(t)
                }
                VaultData::Tracker(tracker) => VaultData::Tracker(tracker.clone()),
            })
            .collect::<Vec<VaultData>>())
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
        /// Recursively searches for the entry in the vault.
        /// `path_index` is the index of the current path element we are looking for.
        fn aux(
            file_entry: VaultData,
            selected_header_path: &[String],
            path_index: usize,
        ) -> Result<VaultData> {
            // Remaining path is empty?
            if path_index == selected_header_path.len() {
                Ok(file_entry)
            } else {
                match &file_entry {
                    VaultData::Header(_, name, children) | VaultData::Directory(name, children) => {
                        if *name == selected_header_path[path_index] {
                            if path_index + 1 == selected_header_path.len() {
                                return Ok(file_entry.clone());
                            }
                            // Look for the child that matches the path
                            for child in children {
                                if let Ok(found) =
                                    aux(child.clone(), selected_header_path, path_index + 1)
                                {
                                    return Ok(found);
                                    // I'm tempted to break here but we might have multiple entries with the same name
                                }
                            }
                        }
                        // Either it's the first layer and the path is wrong or we recursively called on the wrong entry which is impossible
                        bail!("Couldn't find corresponding entry");
                    }
                    VaultData::Task(task) => {
                        if task.name == selected_header_path[path_index] {
                            // If we are at the end of the path, we return the task itself
                            // This depends on the `task_preview_offset` parameter
                            if path_index + 1 == selected_header_path.len() {
                                return Ok(file_entry.clone());
                            }
                            for child in &task.subtasks {
                                if let Ok(found) = aux(
                                    VaultData::Task(child.clone()),
                                    selected_header_path,
                                    path_index + 1,
                                ) {
                                    return Ok(found);
                                }
                            }
                        }
                        bail!("Couldn't find corresponding entry");
                    }
                    VaultData::Tracker(tracker) => {
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

        let filtered_tasks = filter(&self.tasks_refactored, &self.current_filter);
        match filtered_tasks {
            Some(VaultData::Directory(_, entries)) => {
                for entry in entries {
                    if let Ok(res) = aux(entry, path, 0) {
                        return Ok(res);
                    }
                }
                error!("Vault was not empty but the entry was not found");
                bail!("Vault was not empty but the entry was not found");
            }
            None => bail!("Empty Vault"),
            _ => {
                error!("First layer of VaultData was not a Directory");
                bail!("Empty Vault")
            }
        }
    }

    /// Whether the path resolves to something that can be entered or not.
    /// Directories, Headers and Tasks with subtasks can be entered.
    #[must_use]
    pub fn can_enter(&mut self, selected_header_path: &[String]) -> bool {
        fn aux(file_entry: VaultData, selected_header_path: &[String], path_index: usize) -> bool {
            if path_index == selected_header_path.len() {
                true
            } else {
                match file_entry {
                    VaultData::Directory(name, children) | VaultData::Header(_, name, children) => {
                        if name == selected_header_path[path_index] {
                            return children
                                .iter()
                                .any(|c| aux(c.clone(), selected_header_path, path_index + 1));
                        }
                        false
                    }
                    VaultData::Task(task) => {
                        if task.name == selected_header_path[path_index] {
                            return task.subtasks.iter().any(|t| {
                                aux(
                                    VaultData::Task(t.clone()),
                                    selected_header_path,
                                    path_index + 1,
                                )
                            });
                        }
                        false
                    }
                    VaultData::Tracker(_tracker) => false, // Trackers can't be entered at the moment
                                                           // I plan on giving access to its categories someday
                }
            }
        }

        let filtered_tasks = filter(&self.tasks_refactored, &self.current_filter);
        let Some(VaultData::Directory(_, entries)) = filtered_tasks else {
            return false;
        };
        entries
            .iter()
            .any(|e| aux(e.clone(), selected_header_path, 0))
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
    use std::{collections::HashSet, path::PathBuf};

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

        let path = vec![String::from("Test"), String::from("1"), String::from("2")];
        let res = task_mgr.get_vault_data_from_path(&path).unwrap();
        assert_eq!(expected_header, res);

        let path = vec![String::from("Test"), String::from("1.2"), String::from("4")];
        let res = task_mgr.get_vault_data_from_path(&path).unwrap();
        let expected_header = VaultData::Header(2, "4".to_string(), expected_tasks.clone());
        assert_eq!(expected_header, res);
    }
}
