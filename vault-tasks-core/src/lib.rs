use color_eyre::{eyre::bail, Result};
use serde::Deserialize;

use std::{cmp::Ordering, collections::HashSet, fmt::Display, path::PathBuf};
use vault_data::VaultData;

use filter::{filter, Filter};
use tracing::error;
use vault_parser::VaultParser;

pub mod filter;
pub mod parser;
pub mod sorter;
pub mod task;
pub mod vault_data;
mod vault_parser;

pub const FILE_EMOJI: &str = "📄";
pub const DIRECTORY_EMOJI: &str = "📁";
pub const WARNING_EMOJI: &str = "⚠️";

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
    pub vault_path: PathBuf,
    #[serde(default)]
    pub explorer_default_search_string: String,
    #[serde(default)]
    pub filter_default_search_string: String,
    #[serde(default)]
    pub task_state_marker: char,
}

pub struct TaskManager {
    pub tasks: VaultData,
    pub tags: HashSet<String>,
    pub current_filter: Option<Filter>,
}
impl Default for TaskManager {
    fn default() -> Self {
        Self {
            tasks: VaultData::Directory("Empty Vault".to_owned(), vec![]),
            tags: HashSet::new(),
            current_filter: None,
        }
    }
}
impl TaskManager {
    /// Loads a vault from a `Config` and returns a `TaskManager`.
    pub fn load_from_config(config: &TasksConfig) -> Result<Self> {
        let mut res = Self::default();
        res.reload(config)?;
        Ok(res)
    }

    pub fn reload(&mut self, config: &TasksConfig) -> Result<()> {
        let vault_parser = VaultParser::new(config.clone());
        let tasks = vault_parser.scan_vault()?;

        Self::rewrite_vault_tasks(config, &tasks)
            .unwrap_or_else(|e| error!("Failed to fix tasks: {e}"));

        let mut tags = HashSet::new();
        Self::collect_tags(&tasks, &mut tags);

        // debug!("\n{}", tasks);
        // debug!("\n{:#?}", tags);

        self.tasks = tasks;
        self.tags = tags;
        Ok(())
    }
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

    /// Recursively calls `Task.fix_task_attributes` on every task from the vault.
    fn rewrite_vault_tasks(config: &TasksConfig, tasks: &VaultData) -> Result<()> {
        fn explore_tasks_rec(
            config: &TasksConfig,
            filename: &mut PathBuf,
            file_entry: &VaultData,
        ) -> Result<()> {
            match file_entry {
                VaultData::Header(_, _, children) => {
                    children
                        .iter()
                        .try_for_each(|c| explore_tasks_rec(config, filename, c))?;
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

    /// Follows the `selected_header_path` to retrieve the correct `VaultData`.
    /// Then returns a vector of prefixes and a vector of corresponding names from headers or directories to be displayed in TUI.
    /// The method only returns items that are on the same level as the target `VaultData`.
    /// Fails when a task is found or if no corresponding entry is found.
    pub fn get_explorer_entries(
        &self,
        selected_header_path: &[String],
    ) -> Result<Vec<(String, String)>> {
        fn aux(
            file_entry: Vec<VaultData>,
            selected_header_path: &[String],
            path_index: usize,
        ) -> Result<Vec<(String, String)>> {
            if path_index == selected_header_path.len() {
                let mut res = vec![];
                for entry in file_entry {
                    match entry {
                        VaultData::Directory(name, _) => {
                            res.push((
                                if name.contains(".md") {
                                    FILE_EMOJI.to_owned()
                                } else {
                                    DIRECTORY_EMOJI.to_owned()
                                },
                                name.clone(),
                            ));
                        }
                        VaultData::Header(level, name, _) => {
                            res.push(("#".repeat(level).clone(), name));
                        }
                        VaultData::Task(task) => {
                            res.push((task.state.to_string(), task.name));
                        }
                    }
                }
                Ok(res)
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
            Some(VaultData::Directory(_, entries)) => {
                let mut res = aux(entries, selected_header_path, 0)?;

                if let Some(entry) = res.first() {
                    if entry.0 == DIRECTORY_EMOJI || entry.0 == FILE_EMOJI {
                        res.sort_by(|a, b| {
                            if a.0 == DIRECTORY_EMOJI {
                                if b.0 == DIRECTORY_EMOJI {
                                    a.1.cmp(&b.1)
                                } else {
                                    Ordering::Less
                                }
                            } else if b.0 == DIRECTORY_EMOJI {
                                Ordering::Greater
                            } else {
                                a.1.cmp(&b.1)
                            }
                        });
                    }
                }
                Ok(res)
            }
            None => bail!("Empty Vault"),
            _ => {
                error!("First layer of VaultData was not a Directory");
                bail!("First layer of VaultData was not a Directory")
            }
        }
    }

    /// Follows the `selected_header_path` to retrieve the correct `VaultData`.
    /// Returns a vector of `VaultData` with the items to display in TUI, preserving the recursive nature.
    /// `task_preview_offset`: add offset to return a task instead of onne of its subtasks
    pub fn get_vault_data_from_path(
        &self,
        selected_header_path: &[String],
        task_preview_offset: usize,
    ) -> Result<Vec<VaultData>> {
        fn aux(
            file_entry: VaultData,
            selected_header_path: &[String],
            path_index: usize,
            task_preview_offset: usize,
        ) -> Result<Vec<VaultData>> {
            if path_index == selected_header_path.len() {
                Ok(vec![file_entry])
            } else {
                match file_entry {
                    VaultData::Directory(name, children) | VaultData::Header(_, name, children) => {
                        if name == selected_header_path[path_index] {
                            let mut res = vec![];
                            for child in children {
                                if let Ok(mut found) = aux(
                                    child,
                                    selected_header_path,
                                    path_index + 1,
                                    task_preview_offset,
                                ) {
                                    res.append(&mut found);
                                }
                            }
                            Ok(res)
                        } else {
                            bail!("Couldn't find corresponding entry");
                        }
                    }
                    VaultData::Task(task) => {
                        if task.name == selected_header_path[path_index] {
                            let mut res = vec![];

                            if path_index + task_preview_offset == selected_header_path.len() {
                                res.push(VaultData::Task(task));
                            } else {
                                for child in task.subtasks {
                                    if let Ok(mut found) = aux(
                                        VaultData::Task(child),
                                        selected_header_path,
                                        path_index + 1,
                                        task_preview_offset,
                                    ) {
                                        res.append(&mut found);
                                    }
                                }
                            }
                            Ok(res)
                        } else {
                            bail!("Couldn't find corresponding entry");
                        }
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
                    if let Ok(res) = aux(entry, selected_header_path, 0, task_preview_offset) {
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
    fn test_get_entries() {
        let input = VaultData::Directory(
            "test".to_owned(),
            vec![VaultData::Header(
                0,
                "Test".to_string(),
                vec![
                    VaultData::Header(
                        1,
                        "1".to_string(),
                        vec![VaultData::Header(
                            2,
                            "2".to_string(),
                            vec![VaultData::Header(
                                3,
                                "3".to_string(),
                                vec![VaultData::Task(Task {
                                    name: "test".to_string(),
                                    line_number: 8,
                                    description: Some("test\ndesc".to_string()),
                                    ..Default::default()
                                })],
                            )],
                        )],
                    ),
                    VaultData::Header(
                        1,
                        "1.2".to_string(),
                        vec![
                            VaultData::Header(3, "3".to_string(), vec![]),
                            VaultData::Header(
                                2,
                                "4".to_string(),
                                vec![VaultData::Task(Task {
                                    name: "test".to_string(),
                                    line_number: 8,
                                    description: Some("test\ndesc".to_string()),
                                    ..Default::default()
                                })],
                            ),
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
        let expected = vec![(String::from("###"), String::from("3"))];
        let res = task_mgr.get_explorer_entries(&path);
        assert_eq!(expected, res.unwrap());

        let path = vec![String::from("Test"), String::from("1")];
        let expected = vec![(String::from("##"), String::from("2"))];
        let res = task_mgr.get_explorer_entries(&path);
        assert_eq!(expected, res.unwrap());
    }
    #[test]
    fn test_get_entries_err() {
        let input = VaultData::Directory(
            "test".to_owned(),
            vec![VaultData::Header(
                0,
                "Test".to_string(),
                vec![
                    VaultData::Header(
                        1,
                        "1".to_string(),
                        vec![VaultData::Header(
                            2,
                            "2".to_string(),
                            vec![VaultData::Header(
                                3,
                                "3".to_string(),
                                vec![VaultData::Task(Task {
                                    name: "test".to_string(),
                                    line_number: 8,
                                    description: Some("test\ndesc".to_string()),
                                    ..Default::default()
                                })],
                            )],
                        )],
                    ),
                    VaultData::Header(
                        1,
                        "1.2".to_string(),
                        vec![
                            VaultData::Header(3, "3".to_string(), vec![]),
                            VaultData::Header(
                                2,
                                "4".to_string(),
                                vec![VaultData::Task(Task {
                                    name: "test".to_string(),
                                    line_number: 8,
                                    description: Some("test\ndesc".to_string()),
                                    ..Default::default()
                                })],
                            ),
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

        let path = vec![
            String::from("Testaaa"),
            String::from("1"),
            String::from("2"),
        ];
        let res = task_mgr.get_explorer_entries(&path);
        assert!(res.is_err());

        let path = vec![
            String::from("Test"),
            String::from("1"),
            String::from("2"),
            String::from("3"),
        ];
        let res = task_mgr.get_explorer_entries(&path);
        assert_eq!(res.unwrap(), vec![("❌".to_owned(), "test".to_owned())]);
    }

    #[test]
    fn test_get_vault_data() {
        let expected_tasks = vec![
            VaultData::Task(Task {
                name: "test".to_string(),
                line_number: 8,
                description: Some("test\ndesc".to_string()),
                ..Default::default()
            }),
            VaultData::Task(Task {
                name: "test".to_string(),
                line_number: 8,
                description: Some("test\ndesc".to_string()),
                ..Default::default()
            }),
            VaultData::Task(Task {
                name: "test".to_string(),
                line_number: 8,
                description: Some("test\ndesc".to_string()),
                ..Default::default()
            }),
        ];
        let expected_header = VaultData::Header(3, "3".to_string(), expected_tasks.clone());
        let input = VaultData::Directory(
            "test".to_owned(),
            vec![VaultData::Header(
                0,
                "Test".to_string(),
                vec![
                    VaultData::Header(
                        1,
                        "1".to_string(),
                        vec![VaultData::Header(
                            2,
                            "2".to_string(),
                            vec![expected_header.clone()],
                        )],
                    ),
                    VaultData::Header(
                        1,
                        "1.2".to_string(),
                        vec![
                            VaultData::Header(3, "3".to_string(), vec![]),
                            VaultData::Header(
                                2,
                                "4".to_string(),
                                vec![VaultData::Task(Task {
                                    name: "test".to_string(),
                                    line_number: 8,
                                    description: Some("test\ndesc".to_string()),
                                    ..Default::default()
                                })],
                            ),
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
        let res = task_mgr.get_vault_data_from_path(&path, 0).unwrap();
        assert_eq!(vec![expected_header], res);

        let path = vec![
            String::from("Test"),
            String::from("1"),
            String::from("2"),
            String::from("3"),
        ];
        let res = task_mgr.get_vault_data_from_path(&path, 0).unwrap();
        assert_eq!(expected_tasks, res);
    }
}
