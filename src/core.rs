use color_eyre::{eyre::bail, Result};
use serde::Deserialize;
use task::Task;

use std::{
    collections::HashSet,
    fmt::Display,
    path::{Path, PathBuf},
};
use vault_data::VaultData;

use filter::{filter, Filter};
use tracing::{debug, error};
use vault_parser::VaultParser;

pub mod filter;
pub mod parser;
pub mod sorter;
pub mod task;
pub mod vault_data;
mod vault_parser;

#[derive(Clone, Debug, Deserialize)]
pub struct TaskMarkerConfig {
    pub done: char,
    pub todo: char,
    pub incomplete: char,
    pub canceled: char,
}

// Mostly for tests
impl Default for TaskMarkerConfig {
    fn default() -> Self {
        Self {
            done: 'x',
            todo: ' ',
            incomplete: '/',
            canceled: '-',
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct PrettySymbolsConfig {
    #[serde(default)]
    pub task_done: String,
    #[serde(default)]
    pub task_todo: String,
    #[serde(default)]
    pub task_incomplete: String,
    #[serde(default)]
    pub task_canceled: String,
    #[serde(default)]
    pub due_date: String,
    #[serde(default)]
    pub priority: String,
    #[serde(default)]
    pub today_tag: String,
    #[serde(default)]
    pub progress_bar_true: String,
    #[serde(default)]
    pub progress_bar_false: String,
}
impl Default for PrettySymbolsConfig {
    fn default() -> Self {
        Self {
            task_done: String::from("✅"),
            task_todo: String::from("❌"),
            task_incomplete: String::from("⏳"),
            task_canceled: String::from("🚫"),
            due_date: String::from("📅"),
            priority: String::from("❗"),
            today_tag: String::from("☀️"),
            progress_bar_true: String::from("🟩"),
            progress_bar_false: String::from("⬜️"),
        }
    }
}
#[derive(Clone, Debug, Deserialize, Default)]
pub struct TasksConfig {
    #[serde(default)]
    pub parse_dot_files: bool,
    #[serde(default)]
    pub file_tags_propagation: bool,
    #[serde(default)]
    pub ignored: Vec<PathBuf>,
    #[serde(default)]
    pub indent_length: usize,
    #[serde(default)]
    pub use_american_format: bool,
    #[serde(default)]
    pub show_relative_due_dates: bool,
    #[serde(default)]
    pub completion_bar_length: usize,
    #[serde(default)]
    pub vault_path: PathBuf,
    #[serde(default)]
    pub tasks_drop_file: String,
    #[serde(default)]
    pub explorer_default_search_string: String,
    #[serde(default)]
    pub filter_default_search_string: String,
    #[serde(default)]
    pub task_state_markers: TaskMarkerConfig,
    #[serde(default)]
    pub pretty_symbols: PrettySymbolsConfig,
}

pub struct TaskManager {
    pub tasks: VaultData,
    config: TasksConfig,
    pub tags: HashSet<String>,
    pub current_filter: Option<Filter>,
}
impl Default for TaskManager {
    fn default() -> Self {
        Self {
            tasks: VaultData::Directory("Empty Vault".to_owned(), vec![]),
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
        let vault_parser = VaultParser::new(config.clone());
        let tasks = vault_parser.scan_vault()?;

        Self::rewrite_vault_tasks(config, &tasks)
            .unwrap_or_else(|e| error!("Failed to fix tasks: {e}"));

        let mut tags = HashSet::new();
        Self::collect_tags(&tasks, &mut tags);

        self.tasks = tasks;
        self.tags = tags;
        Ok(())
    }

    /// Explores the vault and fills a `&mut HashSet<String>` with every tags found.
    pub fn collect_tags(tasks: &VaultData, tags: &mut HashSet<String>) {
        match tasks {
            VaultData::Directory(_, children) | VaultData::Header(_, _, children) => {
                children.iter().for_each(|c| Self::collect_tags(c, tags));
            }
            VaultData::Task(task) => {
                task.tags.clone().unwrap_or_default().iter().for_each(|t| {
                    tags.insert(t.clone());
                });
                task.subtasks
                    .iter()
                    .for_each(|task| Self::collect_tags(&VaultData::Task(task.clone()), tags));
            }
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
            })
            .collect::<Vec<VaultData>>())
    }

    /// Recursively calls `Task.fix_task_attributes` on every task from the vault.
    fn rewrite_vault_tasks(config: &TasksConfig, tasks: &VaultData) -> Result<()> {
        fn explore_tasks_rec(
            config: &TasksConfig,
            filename: &mut PathBuf,
            file_entry: &VaultData,
        ) -> Result<()> {
            match file_entry {
                VaultData::Header(level, name, children) => {
                    let mut filename = filename.clone();
                    if *level == 0 {
                        filename.push(name);
                    }
                    children
                        .iter()
                        .try_for_each(|c| explore_tasks_rec(config, &mut filename, c))?;
                }
                VaultData::Task(task) => {
                    task.fix_task_attributes(config, filename)?;
                    task.subtasks
                        .iter()
                        .try_for_each(|t| t.fix_task_attributes(config, filename))?;
                }
                VaultData::Directory(dir_name, children) => {
                    let mut filename = filename.clone();
                    filename.push(dir_name);
                    children
                        .iter()
                        .try_for_each(|c| explore_tasks_rec(config, &mut filename.clone(), c))?;
                }
            }
            Ok(())
        }
        explore_tasks_rec(config, &mut PathBuf::new(), tasks)
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
                    // Both variants are very similar
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
                }
            }
        }

        let filtered_tasks = if let Some(task_filter) = &self.current_filter {
            filter(&self.tasks, task_filter)
        } else {
            Some(self.tasks.clone())
        };
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
    pub fn can_enter(&self, selected_header_path: &[String]) -> bool {
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
                }
            }
        }

        let filtered_tasks = if let Some(task_filter) = &self.current_filter {
            filter(&self.tasks, task_filter)
        } else {
            return false;
        };
        let Some(VaultData::Directory(_, entries)) = filtered_tasks else {
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
                VaultData::Directory(name, children) => {
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
            }
        }

        // Get filename
        let filename = &filename_opt.unwrap_or(self.config.tasks_drop_file.clone());
        if filename.is_empty() {
            eprintln!( "No drop file was provided via `--filename`, and no default is set in the configuration." );
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

    use crate::core::{task::Task, vault_data::VaultData};

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
