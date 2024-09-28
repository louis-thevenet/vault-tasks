use color_eyre::{eyre::bail, Result};

use std::{cmp::Ordering, fmt::Display, path::PathBuf};
use vault_data::VaultData;

use tracing::{debug, error};
use vault_parser::VaultParser;

use crate::config::Config;

mod parser;
pub mod task;
pub mod vault_data;
mod vault_parser;

pub const FILE_EMOJI: &str = "📄";
pub const DIRECTORY_EMOJI: &str = "📁";

pub struct TaskManager {
    pub tasks: VaultData,
}
impl Default for TaskManager {
    fn default() -> Self {
        Self {
            tasks: VaultData::Directory("Empty".to_owned(), vec![]),
        }
    }
}
impl TaskManager {
    /// Loads a vault from a `Config` and returns a `TaskManager`.
    pub fn load_from_config(config: &Config) -> Result<Self> {
        let vault_parser = VaultParser::new(config.clone());
        let tasks = vault_parser.scan_vault()?;

        Self::rewrite_vault_tasks(config, &tasks)
            .unwrap_or_else(|e| error!("Failed to fix tasks' due dates: {e}"));

        debug!("\n{}", tasks);
        Ok(Self { tasks })
    }

    /// Recursively calls `Task.fix_task_attributes` on every task from the vault.
    fn rewrite_vault_tasks(config: &Config, tasks: &VaultData) -> Result<()> {
        fn explore_tasks_rec(
            config: &Config,
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
    pub fn get_navigator_entries(
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
                bail!("Error: Couldn't find corresponding entry");
            }
        }

        let VaultData::Directory(_, entries) = self.tasks.clone() else {
            bail!("Error: First layer of VaultData was not a Directory")
        };
        let mut res = aux(entries, selected_header_path, 0)?;
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
        Ok(res)
    }

    /// Follows the `selected_header_path` to retrieve the correct `VaultData`.
    /// Returns a vector of `VaultData` with the items to display in TUI, preserving the recursive nature.
    pub fn get_vault_data_from_path(
        &self,
        selected_header_path: &[String],
    ) -> Result<Vec<VaultData>> {
        fn aux(
            file_entry: VaultData,
            selected_header_path: &[String],
            path_index: usize,
        ) -> Result<Vec<VaultData>> {
            if path_index == selected_header_path.len() {
                Ok(vec![file_entry])
            } else {
                match file_entry {
                    VaultData::Directory(name, children) | VaultData::Header(_, name, children) => {
                        if name == selected_header_path[path_index] {
                            let mut res = vec![];
                            for child in children {
                                if let Ok(mut found) =
                                    aux(child, selected_header_path, path_index + 1)
                                {
                                    res.append(&mut found);
                                }
                            }
                            Ok(res)
                        } else {
                            bail!("Error: Couldn't find corresponding entry");
                        }
                    }
                    VaultData::Task(task) => {
                        if task.name == selected_header_path[path_index] {
                            let mut res = vec![];

                            // Returns early the task to allow previewing its attributes + children
                            if path_index + 1 == selected_header_path.len() {
                                res.push(VaultData::Task(task));
                            } else {
                                for child in task.subtasks {
                                    if let Ok(mut found) = aux(
                                        VaultData::Task(child),
                                        selected_header_path,
                                        path_index + 1,
                                    ) {
                                        res.append(&mut found);
                                    }
                                }
                            }
                            Ok(res)
                        } else {
                            bail!("Error: Couldn't find corresponding entry");
                        }
                    }
                }
            }
        }

        let VaultData::Directory(_, entries) = self.tasks.clone() else {
            bail!("First layer of VaultData was not a Directory")
        };
        for entry in entries {
            if let Ok(res) = aux(entry, selected_header_path, 0) {
                return Ok(res);
            }
        }
        bail!("Error: Couldn't find corresponding file")
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

        let VaultData::Directory(_, entries) = self.tasks.clone() else {
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
    use pretty_assertions::assert_eq;

    use crate::task_core::{task::Task, vault_data::VaultData};

    use super::TaskManager;

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

        let task_mgr = TaskManager { tasks: input };

        let path = vec![String::from("Test"), String::from("1"), String::from("2")];
        let expected = vec![(String::from("###"), String::from("3"))];
        let res = task_mgr.get_navigator_entries(&path);
        assert_eq!(expected, res.unwrap());

        let path = vec![String::from("Test"), String::from("1")];
        let expected = vec![(String::from("##"), String::from("2"))];
        let res = task_mgr.get_navigator_entries(&path);
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

        let task_mgr = TaskManager { tasks: input };

        let path = vec![
            String::from("Testaaa"),
            String::from("1"),
            String::from("2"),
        ];
        let res = task_mgr.get_navigator_entries(&path);
        assert!(res.is_err());

        let path = vec![
            String::from("Test"),
            String::from("1"),
            String::from("2"),
            String::from("3"),
        ];
        let res = task_mgr.get_navigator_entries(&path);
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

        let task_mgr = TaskManager { tasks: input };

        let path = vec![String::from("Test"), String::from("1"), String::from("2")];
        let res = task_mgr.get_vault_data_from_path(&path).unwrap();
        assert_eq!(vec![expected_header], res);

        let path = vec![
            String::from("Test"),
            String::from("1"),
            String::from("2"),
            String::from("3"),
        ];
        let res = task_mgr.get_vault_data_from_path(&path).unwrap();
        assert_eq!(expected_tasks, res);
    }
}
