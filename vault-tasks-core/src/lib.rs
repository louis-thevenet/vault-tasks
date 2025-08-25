use color_eyre::{Result, eyre::bail};
use task::Task;

use std::{
    collections::HashSet,
    fmt::Display,
    path::{self, Path, PathBuf},
};
use vault_data::{FileEntry, Node, VaultData};

use filter::{Filter, filter};
use tracing::{debug, error};
use vault_parser::VaultParser;

use crate::config::TasksConfig;

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
    pub tasks: VaultData,
    config: TasksConfig,
    pub tags: HashSet<String>,
    pub current_filter: Option<Filter>,
}
impl Default for TaskManager {
    fn default() -> Self {
        Self {
            tasks: VaultData::empty(),
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
        if self.config.core.vault_paths.is_empty() {
            bail!( "No vault path provided (use `--vault-path <PATH>`) and no default path set in config file".to_string(), );
        }

        // Check if vault paths exist
        for path in &self.config.core.vault_paths {
            if !Path::new(path).exists() {
                bail!("Vault path does not exist: {path:#?}");
            }
        }

        let vault_parser = VaultParser::new(config.clone());
        let tasks = vault_parser.scan_vault()?;

        Self::rewrite_vault_tasks(config, &tasks)
            .unwrap_or_else(|e| error!("Failed to fix tasks: {e}"));

        let mut tags = HashSet::new();
        tasks
            .root
            .iter()
            .for_each(|node| Self::collect_tags(node, &mut tags));

        self.tasks = tasks;
        self.tags = tags;
        Ok(())
    }

    /// Explores the vault and fills a `&mut HashSet<String>` with every tags found.
    pub fn collect_tags(node: &Node, tags: &mut HashSet<String>) {
        fn collect_tags_aux(entry: &FileEntry, tags: &mut HashSet<String>) {
            match entry {
                FileEntry::Task(task) => {
                    task.tags.clone().unwrap_or_default().iter().for_each(|t| {
                        tags.insert(t.clone());
                    });
                    task.subtasks
                        .iter()
                        .for_each(|task| collect_tags_aux(&FileEntry::Task(task.clone()), tags));
                }
                FileEntry::Tracker(_tracker) => (),
                FileEntry::Header {
                    name: _,
                    heading_level: _,
                    content,
                } => content
                    .iter()
                    .for_each(|entry| collect_tags_aux(entry, tags)),
            }
        }
        match node {
            Node::Directory {
                name: _,
                path: _,
                content: children,
            }
            | Node::Vault {
                name: _,
                path: _,
                content: children,
            } => {
                children.iter().for_each(|c| Self::collect_tags(c, tags));
            }
            Node::File {
                name: _,
                path: _,
                content,
            } => content
                .iter()
                .for_each(|entry| collect_tags_aux(entry, tags)),
        }
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
                        VaultData::Root { vaults } => {
                            return aux(vaults, selected_header_path, path_index);
                        }
                        VaultData::Directory(name, children)
                        | VaultData::Header(_, name, children)
                        | VaultData::Vault {
                            short_name: name,
                            content: children,
                            path: _,
                        } => {
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

        let filtered_tasks = if let Some(task_filter) = &self.current_filter {
            filter(&self.tasks, task_filter)
        } else {
            Some(self.tasks.clone())
        };

        match filtered_tasks {
            Some(VaultData::Root { vaults }) => aux(vaults, selected_header_path, 0),
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
                VaultData::Root { vaults: _ } => VaultData::Root { vaults: vec![] },
                VaultData::Vault {
                    short_name,
                    path,
                    content: _,
                } => VaultData::Vault {
                    short_name: short_name.clone(),
                    path: path.clone(),
                    content: vec![],
                },

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
    fn rewrite_vault_tasks(config: &TasksConfig, tasks: &VaultData) -> Result<()> {
        fn explore_file_entry_rec(config: &TasksConfig, file_entry: &FileEntry) -> Result<()> {
            match file_entry {
                FileEntry::Header {
                    heading_level: _,
                    name: _,
                    content: children,
                } => {
                    children
                        .iter()
                        .try_for_each(|c| explore_file_entry_rec(config, c))?;
                }
                FileEntry::Task(task) => {
                    task.fix_task_attributes(config)?;
                    task.subtasks
                        .iter()
                        .try_for_each(|t| t.fix_task_attributes(config))?;
                }
                FileEntry::Tracker(tracker) => tracker.fix_tracker_attributes(config)?,
            }
            Ok(())
        }
        fn explore_tasks_rec(config: &TasksConfig, node: &Node) -> Result<()> {
            match node {
                Node::Vault {
                    name: _,
                    path,
                    content,
                } => {
                    content
                        .iter()
                        .try_for_each(|c| explore_tasks_rec(config, c))?;
                }
                Node::Directory {
                    name: dir_name,
                    path: _,
                    content: children,
                } => {
                    children
                        .iter()
                        .try_for_each(|c| explore_tasks_rec(config, c))?;
                }
                Node::File {
                    name: _,
                    path: _,
                    content,
                } => {
                    for file_entry in content {
                        explore_file_entry_rec(config, file_entry)?;
                    }
                }
            }
            Ok(())
        }
        for node in tasks.root.iter() {
            explore_tasks_rec(config, node)?
        }
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
                    VaultData::Root { vaults } => {
                        for vault in vaults {
                            if let Ok(found) = aux(vault.clone(), selected_header_path, path_index)
                            {
                                return Ok(found);
                            }
                        }
                        bail!("Couldn't find corresponding entry");
                    }

                    VaultData::Header(_, name, children)
                    | VaultData::Directory(name, children)
                    | VaultData::Vault {
                        short_name: name,
                        path: _,
                        content: children,
                    } => {
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

        let filtered_tasks = if let Some(task_filter) = &self.current_filter {
            filter(&self.tasks, task_filter)
        } else {
            Some(self.tasks.clone())
        };
        match filtered_tasks {
            Some(VaultData::Directory(_, entries) | VaultData::Root { vaults: entries }) => {
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
                    VaultData::Root { vaults } => {
                        debug!("Checking if path can be entered in the root vaults");
                        // If we are at the root, we can enter it
                        if path_index == 0 {
                            return true;
                        }
                        vaults
                            .iter()
                            .any(|v| aux(v.clone(), selected_header_path, path_index))
                    }
                    VaultData::Directory(name, children)
                    | VaultData::Header(_, name, children)
                    | VaultData::Vault {
                        short_name: name,
                        path: _,
                        content: children,
                    } => {
                        debug!("Checking if path can be entered in Directory or Header: {name}");
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

        let filtered_tasks = if let Some(task_filter) = &self.current_filter {
            debug!("filter?");
            filter(&self.tasks, task_filter)
        } else {
            debug!("No filter");
            return false;
        };
        let Some(VaultData::Root { vaults: entries }) = filtered_tasks else {
            debug!("Filtered tasks were not a Directory");
            return false;
        };
        entries
            .iter()
            .any(|e| aux(e.clone(), selected_header_path, 0))
    }

    pub fn add_task(&mut self, task_desc: &str, filename_opt: Option<String>) {
        /// Recursively searches for the entry in the vault.
        /// `path_index` is the index of the current path element we are looking for.
        fn aux(file_entry: &mut VaultData, filename: &str, new_task: &Task) -> Result<()> {
            match file_entry {
                VaultData::Root { vaults } => {
                    for vault in vaults {
                        if let Ok(()) = aux(vault, filename, new_task) {
                            return Ok(());
                        }
                    }
                    bail!("Couldn't find corresponding entry in vaults");
                }
                VaultData::Directory(name, children)
                | VaultData::Vault {
                    short_name: name,
                    path: _,
                    content: children,
                } => {
                    // Look for the child that matches the path
                    for child in children {
                        if let Ok(()) = aux(child, filename, new_task) {
                            return Ok(());
                            // I'm tempted to break here but we might have multiple entries with the same name
                        }
                    }
                    // Either it's the first layer and the path is wrong or we recursively called on the wrong entry which is impossible
                    bail!("Couldn't find corresponding entry in Directory {name}");
                }
                VaultData::Header(_, name, children) => {
                    if *name == filename {
                        debug! {"Adding task to {name}"
                        };

                        children.push(VaultData::Task(new_task.clone()));
                        return Ok(());
                    }
                    // Look for the child that matches the path
                    for child in children {
                        if let Ok(()) = aux(child, filename, new_task) {
                            return Ok(());
                            // I'm tempted to break here but we might have multiple entries with the same name
                        }
                    }
                    // Either it's the first layer and the path is wrong or we recursively called on the wrong entry which is impossible
                    bail!("Couldn't find corresponding entry in Header {name}");
                }
                VaultData::Task(_task) => {
                    bail!("Adding subtasks from CLI is not supported yet");
                }
                VaultData::Tracker(_tracker) => {
                    bail!("Tried to add a task to a tracker")
                }
            }
        }

        // Get filename
        let filename = &filename_opt.unwrap_or(self.config.core.tasks_drop_file.clone());
        if filename.is_empty() {
            eprintln!(
                "No drop file was provided via `--filename`, and no default is set in the configuration."
            );
            return;
        }
        if !Path::new(&filename)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        {
            eprintln!("Filename {filename} does not have the .md extension");
            return;
        }

        // Parse input string
        let vault_parser = VaultParser::new(self.config.clone());
        let task = match vault_parser.parse_single_task(task_desc, filename) {
            Ok(task) => task,
            Err(e) => {
                eprintln!("Failed to parse task: {e}");
                return;
            }
        };
        debug!("Adding new task: {} to path: {:?}", task, filename);

        // Insert the task into the vault tree
        if let Err(e) = aux(&mut self.tasks, filename, &task) {
            eprintln!("Failed to insert task in VaultData tree: {e}");
            return;
        }

        // Fix attributes again (maybe we should only fix the task itself
        // but we would need the path to filename)
        if let Err(e) = Self::rewrite_vault_tasks(&self.config, &self.tasks) {
            eprintln!("Failed to fix task attributes in vault files: {e}");
        }
        if let Err(e) = self.reload(&self.config.clone()) {
            eprintln!("Failed to reload tasks after adding a new task: {e}");
        } else {
            debug!("Successfully added task to vault");
        }
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

    use crate::{task::Task, vault_data::VaultData};

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

        let task_mgr = TaskManager {
            tasks: input,
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
