use std::path::PathBuf;

use crate::{
    TasksConfig,
    vault_data::{FileEntryNode, VaultNode, Vaults},
};

use super::{
    date::Date,
    parser::task::parse_task,
    task::{State, Task},
};

#[derive(Default, PartialEq, Eq, Debug)]
pub struct Filter {
    pub task: Task,
    inverted: bool,
    /// Separate state in case we're not filtering by state
    /// Since tasks reuire a state to be valid, we'll add a default one but use this one
    state: Option<State>,
}

/// Parses a [`Task`] from an input `&str`. Returns a [`Filter`] object.
#[must_use]
pub fn parse_search_input(input: &str, config: &TasksConfig) -> Filter {
    // Invert results ? If so, remove the `!` prefix
    let inverted = input.starts_with('!');
    let input = input.strip_prefix('!').unwrap_or(input);

    // Is a state specified ? If not, add a default state to make it parseable but we won't take it into account
    let has_state = input.starts_with("- [");
    let input_value = format!("{}{}", if has_state { "" } else { "- [ ]" }, input);

    // Parse the input
    let task = match parse_task(&mut input_value.as_str(), &PathBuf::new(), config) {
        Ok(t) => t,
        Err(_e) => Task {
            name: String::from("Incomplete search prompt"),
            ..Default::default()
        },
    };
    Filter {
        task: task.clone(),
        inverted,
        state: if has_state { Some(task.state) } else { None },
    }
}

fn filter_task(task: &Task, filter: &Filter) -> bool {
    let state_match = filter.state.is_none()
        || filter.state.clone().is_some_and(|state| {
            // This is not really satisfying as you can't
            // match only incomplete tasks for example
            // I should add a config option for it
            matches!(
                (state, &task.state),
                (
                    State::ToDo | State::Incomplete,
                    State::ToDo | State::Incomplete
                ) | (State::Done | State::Canceled, State::Done | State::Canceled)
            )
        });

    let name_match = names_match(&task.name, &filter.task.name);

    let today_flag_match = if filter.task.is_today {
        task.is_today
    } else {
        true
    };

    let date_match = match (task.due_date.clone(), filter.task.due_date.clone()) {
        (_, None) => true,
        (Some(Date::DayTime(task_date)), Some(Date::DayTime(search_date)))
            if task_date == search_date =>
        {
            true
        }
        (Some(Date::Day(task_date)), Some(Date::Day(search_date))) if task_date == search_date => {
            true
        }
        (_, _) => false,
    };

    let tags_match = filter
        .task
        .tags
        .clone()
        .unwrap_or_default()
        .iter()
        .all(|t| {
            task.tags
                .clone()
                .unwrap_or_default()
                .iter()
                .any(|x| x.to_lowercase().contains(&t.to_lowercase()))
        });

    let priority_match = if filter.task.priority > 0 {
        filter.task.priority == task.priority
    } else {
        true
    };
    filter.inverted
        ^ (state_match
            && name_match
            && today_flag_match
            && date_match
            && tags_match
            && priority_match)
}

fn names_match(name: &str, filter_name: &str) -> bool {
    if filter_name.is_empty() {
        true
    } else {
        // for each word of the filter_task, if at least one
        // matches in the task, then validate
        filter_name
            .to_lowercase()
            .split_whitespace()
            .filter(|w| name.to_lowercase().contains(w))
            .count()
            > 0
    }
}

/// Will return a `Vec<Task>` matching the given `Filter` from the `VaultData`. Includes subtasks.
#[must_use]
pub fn filter_tasks_to_vec(vault_data: &Vaults, filter: &Filter) -> Vec<Task> {
    fn filter_tasks_from_file_entry(
        file_entry: FileEntryNode,
        filter: &Filter,
        res: &mut Vec<Task>,
    ) {
        match file_entry {
            FileEntryNode::Header { content, .. } => {
                for c in content {
                    filter_tasks_from_file_entry(c, filter, res);
                }
            }
            FileEntryNode::Task(task) => {
                if filter_task(&task, filter) {
                    res.push(task.clone());
                }
                task.subtasks.iter().for_each(|t| {
                    filter_tasks_from_file_entry(FileEntryNode::Task(t.clone()), filter, res);
                });
            }
            FileEntryNode::Tracker(_tracker) => {} // Don't collect trackers in the result
                                                   // It's only used by the Filter and Calendar
                                                   // tabs and we don't want to display trackers there
        }
    }
    fn filter_tasks_from_node(node: &VaultNode, filter: &Filter, res: &mut Vec<Task>) {
        match node {
            VaultNode::Vault { content, .. } | VaultNode::Directory { content, .. } => {
                for entry in content {
                    filter_tasks_from_node(entry, filter, res);
                }
            }
            VaultNode::File { content, .. } => {
                for entry in content.clone() {
                    filter_tasks_from_file_entry(entry, filter, res);
                }
            }
        }
    }
    let res = &mut vec![];
    vault_data.root.iter().for_each(|node| {
        filter_tasks_from_node(node, filter, res);
    });
    res.clone()
}

/// Filters a `VaultData` structure based on the provided `Filter`.
/// Only keeps the `VaultData` entries that match the filter criteria.
#[must_use]
pub fn filter(vault_data: &Vaults, task_filter: &Option<Filter>) -> Option<Vaults> {
    fn filter_file_entry(
        file_entry: &FileEntryNode,
        task_filter: &Filter,
    ) -> Option<FileEntryNode> {
        match &file_entry {
            FileEntryNode::Header {
                content,
                heading_level,
                name,
            } => {
                let mut actual_content = vec![];
                for child in content {
                    let child_clone = child.clone();
                    if let Some(child) = filter_file_entry(&child_clone, task_filter) {
                        actual_content.push(child);
                    }
                }
                if actual_content.is_empty() {
                    None
                } else {
                    Some(FileEntryNode::Header {
                        heading_level: *heading_level,
                        name: name.clone(),
                        content: actual_content,
                    })
                }
            }
            FileEntryNode::Task(task) => {
                if filter_task(task, task_filter) {
                    Some(file_entry.clone())
                } else {
                    let mut actual_children = vec![];
                    for child in &task.subtasks {
                        if let Some(FileEntryNode::Task(child)) =
                            filter_file_entry(&FileEntryNode::Task(child.clone()), task_filter)
                        {
                            actual_children.push(child);
                        }
                    }
                    if actual_children.is_empty() {
                        return None;
                    }
                    Some(FileEntryNode::Task(Task {
                        subtasks: actual_children,
                        ..task.clone()
                    }))
                }
            }
            FileEntryNode::Tracker(tracker) => {
                // We keep the tracker if its name matches the filter task's name
                // But we don't look at the task's state
                // I might want to refactor the Filter to allow parsing a Tracker from
                // the input string later.
                if names_match(&tracker.name, &task_filter.task.name) {
                    Some(FileEntryNode::Tracker(tracker.clone()))
                } else {
                    None
                }
            }
        }
    }
    fn filter_node(node: &VaultNode, filter: &Filter) -> Option<VaultNode> {
        match node {
            VaultNode::Vault {
                content,
                name,
                path,
            } => {
                let mut actual_children = vec![];
                for child in content {
                    let child_clone = child.clone();
                    if let Some(child) = filter_node(&child_clone, filter) {
                        actual_children.push(child);
                    }
                }
                if actual_children.is_empty() {
                    None
                } else {
                    Some(VaultNode::Vault {
                        name: name.to_string(),
                        content: actual_children,
                        path: path.clone(),
                    })
                }
            }
            VaultNode::Directory {
                content,
                name,
                path,
            } => {
                let mut actual_children = vec![];
                for child in content {
                    let child_clone = child.clone();
                    if let Some(child) = filter_node(&child_clone, filter) {
                        actual_children.push(child);
                    }
                }
                if actual_children.is_empty() {
                    None
                } else {
                    Some(VaultNode::Directory {
                        name: name.to_string(),
                        content: actual_children,
                        path: path.clone(),
                    })
                }
            }
            VaultNode::File {
                name,
                path,
                content,
            } => {
                let mut actual_children = vec![];
                for child in content {
                    let child_clone = child.clone();
                    if let Some(child) = filter_file_entry(&child_clone, filter) {
                        actual_children.push(child);
                    }
                }
                if actual_children.is_empty() {
                    None
                } else {
                    Some(VaultNode::File {
                        name: name.to_string(),
                        content: actual_children,
                        path: path.clone(),
                    })
                }
            }
        }
    }
    let Some(task_filter) = task_filter else {
        return Some(vault_data.clone());
    };
    let mut actual_children = vec![];
    for node in &vault_data.root {
        let node_clone = node.clone();
        if let Some(child) = filter_node(&node_clone, task_filter) {
            actual_children.push(child);
        }
    }
    if actual_children.is_empty() {
        None
    } else {
        Some(Vaults::new(actual_children))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::NaiveDate;

    use crate::{
        TasksConfig,
        date::Date,
        filter::{Filter, filter},
        task::{State, Task},
        vault_data::{FileEntryNode, VaultNode, Vaults},
    };

    use super::{filter_tasks_to_vec, parse_search_input};

    #[test]
    fn parse_search_input_test() {
        let input = "- [ ] #tag today name p5";
        let config = TasksConfig::default();
        let res = parse_search_input(input, &config);
        let expected = Filter {
            task: Task {
                due_date: Some(Date::Day(chrono::Local::now().date_naive())),
                name: String::from("name"),
                priority: 5,
                state: State::ToDo,
                tags: Some(vec![String::from("tag")]),
                ..Default::default()
            },
            inverted: false,
            state: Some(State::ToDo),
        };
        assert_eq!(expected, res);
    }

    #[test]
    fn parse_search_input_no_state_test() {
        let input = "#tag today name p5";
        let config = TasksConfig::default();
        let res = parse_search_input(input, &config);
        let expected = Filter {
            task: Task {
                due_date: Some(Date::Day(chrono::Local::now().date_naive())),
                name: String::from("name"),
                priority: 5,
                state: State::ToDo,
                tags: Some(vec![String::from("tag")]),
                ..Default::default()
            },
            inverted: false,
            state: None,
        };
        assert_eq!(expected, res);
    }

    #[test]
    fn parse_search_input_inverted_test() {
        let input = "!- [ ] #tag today name p5";
        let config = TasksConfig::default();
        let res = parse_search_input(input, &config);
        let expected = Filter {
            task: Task {
                due_date: Some(Date::Day(chrono::Local::now().date_naive())),
                name: String::from("name"),
                priority: 5,
                state: State::ToDo,
                tags: Some(vec![String::from("tag")]),
                ..Default::default()
            },
            inverted: true,
            state: Some(State::ToDo),
        };
        assert_eq!(expected, res);
    }

    #[test]
    fn parse_search_input_inverted_no_state_test() {
        let input = "!#tag today name p5";
        let config = TasksConfig::default();
        let res = parse_search_input(input, &config);
        let expected = Filter {
            task: Task {
                due_date: Some(Date::Day(chrono::Local::now().date_naive())),
                name: String::from("name"),
                priority: 5,
                state: State::ToDo,
                tags: Some(vec![String::from("tag")]),
                ..Default::default()
            },
            inverted: true,
            state: None,
        };
        assert_eq!(expected, res);
    }

    #[test]
    fn filter_tags_test() {
        let input = Vaults::new(vec![VaultNode::Directory {
            name: "test".to_owned(),
            path: PathBuf::from("test"),
            content: vec![VaultNode::File {
                name: "Test".to_string(),
                path: PathBuf::from("test/Test"),
                content: vec![
                    FileEntryNode::Header {
                        name: "1".to_string(),
                        heading_level: 1,
                        content: vec![FileEntryNode::Header {
                            name: "2".to_string(),
                            heading_level: 2,
                            content: vec![FileEntryNode::Task(Task {
                                name: "test 1".to_string(),
                                line_number: Some(8),
                                description: Some("test\ndesc".to_string()),
                                ..Default::default()
                            })],
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
                                content: vec![FileEntryNode::Task(Task {
                                    name: "test 2".to_string(),
                                    line_number: Some(8),
                                    tags: Some(vec!["test".to_string()]),
                                    description: Some("test\ndesc".to_string()),
                                    ..Default::default()
                                })],
                            },
                        ],
                    },
                    FileEntryNode::Task(Task {
                        name: "test 3".to_string(),
                        line_number: Some(8),
                        tags: Some(vec!["test".to_string()]),
                        description: Some("test\ndesc".to_string()),
                        ..Default::default()
                    }),
                ],
            }],
        }]);
        let expected = vec![
            Task {
                name: "test 2".to_string(),
                line_number: Some(8),
                tags: Some(vec!["test".to_string()]),
                description: Some("test\ndesc".to_string()),
                ..Default::default()
            },
            Task {
                name: "test 3".to_string(),
                line_number: Some(8),
                tags: Some(vec!["test".to_string()]),
                description: Some("test\ndesc".to_string()),
                ..Default::default()
            },
        ];
        let res = filter_tasks_to_vec(
            &input,
            &Filter {
                task: Task {
                    name: String::new(),
                    tags: Some(vec!["test".to_string()]),
                    ..Default::default()
                },
                inverted: false,
                state: None,
            },
        );
        assert_eq!(res, expected);
    }

    #[test]
    fn filter_tags_inverted_test() {
        let input = Vaults::new(vec![VaultNode::Directory {
            name: "test".to_owned(),
            path: PathBuf::from("test"),
            content: vec![VaultNode::File {
                name: "Test".to_string(),
                path: PathBuf::from("test/Test"),
                content: vec![
                    FileEntryNode::Header {
                        name: "1".to_string(),
                        heading_level: 1,
                        content: vec![FileEntryNode::Header {
                            name: "2".to_string(),
                            heading_level: 2,
                            content: vec![FileEntryNode::Task(Task {
                                name: "test 1".to_string(),
                                line_number: Some(8),
                                description: Some("test\ndesc".to_string()),
                                ..Default::default()
                            })],
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
                                content: vec![FileEntryNode::Task(Task {
                                    name: "test 2".to_string(),
                                    line_number: Some(8),
                                    tags: Some(vec!["test".to_string()]),
                                    description: Some("test\ndesc".to_string()),
                                    ..Default::default()
                                })],
                            },
                        ],
                    },
                    FileEntryNode::Task(Task {
                        name: "test 3".to_string(),
                        line_number: Some(8),
                        tags: Some(vec!["test".to_string()]),
                        description: Some("test\ndesc".to_string()),
                        ..Default::default()
                    }),
                ],
            }],
        }]);
        let expected = vec![Task {
            name: "test 1".to_string(),
            line_number: Some(8),
            description: Some("test\ndesc".to_string()),
            ..Default::default()
        }];
        let res = filter_tasks_to_vec(
            &input,
            &Filter {
                task: Task {
                    name: String::new(),
                    tags: Some(vec!["test".to_string()]),
                    ..Default::default()
                },
                inverted: true,
                state: None,
            },
        );
        assert_eq!(res, expected);
    }

    #[test]
    fn filter_names_test() {
        let input = Vaults::new(vec![VaultNode::Directory {
            name: "test".to_owned(),
            path: PathBuf::from("test"),
            content: vec![VaultNode::File {
                name: "Test".to_string(),
                path: PathBuf::from("test/Test"),
                content: vec![
                    FileEntryNode::Header {
                        name: "1".to_string(),
                        heading_level: 1,
                        content: vec![FileEntryNode::Header {
                            name: "2".to_string(),
                            heading_level: 2,
                            content: vec![FileEntryNode::Task(Task {
                                name: "hfdgqskhjfg1".to_string(),
                                line_number: Some(8),
                                description: Some("test\ndesc".to_string()),
                                ..Default::default()
                            })],
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
                                content: vec![FileEntryNode::Task(Task {
                                    name: "test 2".to_string(),
                                    line_number: Some(8),
                                    tags: Some(vec!["test".to_string()]),
                                    description: Some("test\ndesc".to_string()),
                                    ..Default::default()
                                })],
                            },
                        ],
                    },
                    FileEntryNode::Task(Task {
                        name: "test 3".to_string(),
                        line_number: Some(8),
                        tags: Some(vec!["test".to_string()]),
                        description: Some("test\ndesc".to_string()),
                        ..Default::default()
                    }),
                ],
            }],
        }]);
        let expected = vec![
            Task {
                name: "test 2".to_string(),
                line_number: Some(8),
                tags: Some(vec!["test".to_string()]),
                description: Some("test\ndesc".to_string()),
                ..Default::default()
            },
            Task {
                name: "test 3".to_string(),
                line_number: Some(8),
                tags: Some(vec!["test".to_string()]),
                description: Some("test\ndesc".to_string()),
                ..Default::default()
            },
        ];
        let res = filter_tasks_to_vec(
            &input,
            &Filter {
                task: Task {
                    name: String::from("test"),
                    ..Default::default()
                },
                inverted: false,
                state: None,
            },
        );
        assert_eq!(res, expected);
    }

    #[test]
    fn filter_names_inverted_test() {
        let input = Vaults::new(vec![VaultNode::Directory {
            name: "test".to_owned(),
            path: PathBuf::from("test"),
            content: vec![VaultNode::File {
                name: "Test".to_string(),
                path: PathBuf::from("test/Test"),
                content: vec![
                    FileEntryNode::Header {
                        name: "1".to_string(),
                        heading_level: 1,
                        content: vec![FileEntryNode::Header {
                            name: "2".to_string(),
                            heading_level: 2,
                            content: vec![FileEntryNode::Task(Task {
                                name: "hfdgqskhjfg1".to_string(),
                                line_number: Some(8),
                                description: Some("test\ndesc".to_string()),
                                ..Default::default()
                            })],
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
                                content: vec![FileEntryNode::Task(Task {
                                    name: "test 2".to_string(),
                                    line_number: Some(8),
                                    tags: Some(vec!["test".to_string()]),
                                    description: Some("test\ndesc".to_string()),
                                    ..Default::default()
                                })],
                            },
                        ],
                    },
                    FileEntryNode::Task(Task {
                        name: "test 3".to_string(),
                        line_number: Some(8),
                        tags: Some(vec!["test".to_string()]),
                        description: Some("test\ndesc".to_string()),
                        ..Default::default()
                    }),
                ],
            }],
        }]);
        let expected = vec![Task {
            name: "hfdgqskhjfg1".to_string(),
            line_number: Some(8),
            description: Some("test\ndesc".to_string()),
            ..Default::default()
        }];
        let res = filter_tasks_to_vec(
            &input,
            &Filter {
                task: Task {
                    name: String::from("test"),
                    ..Default::default()
                },
                inverted: true,
                state: None,
            },
        );
        assert_eq!(res, expected);
    }

    #[test]
    fn filter_due_date_test() {
        let input = Vaults::new(vec![VaultNode::Directory {
            name: "test".to_owned(),
            path: PathBuf::from("test"),
            content: vec![VaultNode::File {
                name: "Test".to_string(),
                path: PathBuf::from("test/Test"),
                content: vec![
                    FileEntryNode::Header {
                        name: "1".to_string(),
                        heading_level: 1,
                        content: vec![FileEntryNode::Header {
                            name: "2".to_string(),
                            heading_level: 2,
                            content: vec![FileEntryNode::Task(Task {
                                name: "hfdgqskhjfg1".to_string(),
                                line_number: Some(8),
                                due_date: Some(Date::Day(
                                    NaiveDate::from_ymd_opt(2020, 2, 2).unwrap(),
                                )),
                                description: Some("test\ndesc".to_string()),
                                ..Default::default()
                            })],
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
                                content: vec![FileEntryNode::Task(Task {
                                    name: "test 2".to_string(),
                                    line_number: Some(8),
                                    tags: Some(vec!["test".to_string()]),
                                    description: Some("test\ndesc".to_string()),
                                    ..Default::default()
                                })],
                            },
                        ],
                    },
                    FileEntryNode::Task(Task {
                        name: "test 3".to_string(),
                        line_number: Some(8),
                        tags: Some(vec!["test".to_string()]),
                        description: Some("test\ndesc".to_string()),
                        ..Default::default()
                    }),
                ],
            }],
        }]);
        let expected = vec![Task {
            name: "hfdgqskhjfg1".to_string(),
            line_number: Some(8),
            due_date: Some(Date::Day(NaiveDate::from_ymd_opt(2020, 2, 2).unwrap())),
            description: Some("test\ndesc".to_string()),
            ..Default::default()
        }];
        let res = filter_tasks_to_vec(
            &input,
            &Filter {
                task: Task {
                    due_date: Some(Date::Day(NaiveDate::from_ymd_opt(2020, 2, 2).unwrap())),
                    ..Default::default()
                },
                inverted: false,
                state: None,
            },
        );
        assert_eq!(res, expected);
    }

    #[test]
    fn filter_due_date_inverted_test() {
        let input = Vaults::new(vec![VaultNode::Directory {
            name: "test".to_owned(),
            path: PathBuf::from("test"),
            content: vec![VaultNode::File {
                name: "Test".to_string(),
                path: PathBuf::from("test/Test"),
                content: vec![
                    FileEntryNode::Header {
                        name: "1".to_string(),
                        heading_level: 1,
                        content: vec![FileEntryNode::Header {
                            name: "2".to_string(),
                            heading_level: 2,
                            content: vec![FileEntryNode::Task(Task {
                                name: "hfdgqskhjfg1".to_string(),
                                line_number: Some(8),
                                due_date: Some(Date::Day(
                                    NaiveDate::from_ymd_opt(2020, 2, 2).unwrap(),
                                )),
                                description: Some("test\ndesc".to_string()),
                                ..Default::default()
                            })],
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
                                content: vec![FileEntryNode::Task(Task {
                                    name: "test 2".to_string(),
                                    line_number: Some(8),
                                    tags: Some(vec!["test".to_string()]),
                                    description: Some("test\ndesc".to_string()),
                                    ..Default::default()
                                })],
                            },
                        ],
                    },
                    FileEntryNode::Task(Task {
                        name: "test 3".to_string(),
                        line_number: Some(8),
                        tags: Some(vec!["test".to_string()]),
                        description: Some("test\ndesc".to_string()),
                        ..Default::default()
                    }),
                ],
            }],
        }]);
        let expected = vec![
            Task {
                name: "test 2".to_string(),
                line_number: Some(8),
                tags: Some(vec!["test".to_string()]),
                description: Some("test\ndesc".to_string()),
                ..Default::default()
            },
            Task {
                name: "test 3".to_string(),
                line_number: Some(8),
                tags: Some(vec!["test".to_string()]),
                description: Some("test\ndesc".to_string()),
                ..Default::default()
            },
        ];
        let res = filter_tasks_to_vec(
            &input,
            &Filter {
                task: Task {
                    due_date: Some(Date::Day(NaiveDate::from_ymd_opt(2020, 2, 2).unwrap())),
                    ..Default::default()
                },
                inverted: true,
                state: None,
            },
        );
        assert_eq!(res, expected);
    }

    #[test]
    fn filter_full_test() {
        let input = Vaults::new(vec![VaultNode::Directory {
            name: "test".to_owned(),
            path: PathBuf::from("test"),
            content: vec![VaultNode::File {
                name: "Test".to_string(),
                path: PathBuf::from("test/Test"),
                content: vec![
                    FileEntryNode::Header {
                        name: "1".to_string(),
                        heading_level: 1,
                        content: vec![FileEntryNode::Header {
                            name: "2".to_string(),
                            heading_level: 2,
                            content: vec![FileEntryNode::Task(Task {
                                name: "real target".to_string(),
                                line_number: Some(8),
                                tags: Some(vec!["test".to_string()]),
                                due_date: Some(Date::Day(
                                    NaiveDate::from_ymd_opt(2020, 2, 2).unwrap(),
                                )),
                                description: Some("test\ndesc".to_string()),
                                ..Default::default()
                            })],
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
                                content: vec![FileEntryNode::Task(Task {
                                    name: "false target 2".to_string(),
                                    line_number: Some(8),
                                    tags: Some(vec!["test".to_string()]),
                                    description: Some("test\ndesc".to_string()),
                                    ..Default::default()
                                })],
                            },
                        ],
                    },
                    FileEntryNode::Task(Task {
                        name: "test 3".to_string(),
                        line_number: Some(8),
                        tags: Some(vec!["test".to_string()]),
                        description: Some("test\ndesc".to_string()),
                        ..Default::default()
                    }),
                ],
            }],
        }]);
        let expected = vec![Task {
            name: "real target".to_string(),
            line_number: Some(8),
            due_date: Some(Date::Day(NaiveDate::from_ymd_opt(2020, 2, 2).unwrap())),
            tags: Some(vec!["test".to_string()]),
            description: Some("test\ndesc".to_string()),
            ..Default::default()
        }];
        let res = filter_tasks_to_vec(
            &input,
            &Filter {
                task: Task {
                    name: String::from("target"),
                    tags: Some(vec!["test".to_string()]),
                    due_date: Some(Date::Day(NaiveDate::from_ymd_opt(2020, 2, 2).unwrap())),
                    ..Default::default()
                },
                inverted: false,
                state: None,
            },
        );
        assert_eq!(res, expected);
    }

    #[test]
    fn filter_full_inverted_test() {
        let input = Vaults::new(vec![VaultNode::Directory {
            name: "test".to_owned(),
            path: PathBuf::from("test"),
            content: vec![VaultNode::File {
                name: "Test".to_string(),
                path: PathBuf::from("test/Test"),
                content: vec![
                    FileEntryNode::Header {
                        name: "1".to_string(),
                        heading_level: 1,
                        content: vec![FileEntryNode::Header {
                            name: "2".to_string(),
                            heading_level: 2,
                            content: vec![FileEntryNode::Task(Task {
                                name: "real target".to_string(),
                                line_number: Some(8),
                                tags: Some(vec!["test".to_string()]),
                                due_date: Some(Date::Day(
                                    NaiveDate::from_ymd_opt(2020, 2, 2).unwrap(),
                                )),
                                description: Some("test\ndesc".to_string()),
                                ..Default::default()
                            })],
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
                                content: vec![FileEntryNode::Task(Task {
                                    name: "false target 2".to_string(),
                                    line_number: Some(8),
                                    tags: Some(vec!["test".to_string()]),
                                    description: Some("test\ndesc".to_string()),
                                    ..Default::default()
                                })],
                            },
                        ],
                    },
                    FileEntryNode::Task(Task {
                        name: "test 3".to_string(),
                        line_number: Some(8),
                        tags: Some(vec!["test".to_string()]),
                        description: Some("test\ndesc".to_string()),
                        ..Default::default()
                    }),
                ],
            }],
        }]);
        let expected = vec![
            Task {
                name: "false target 2".to_string(),
                line_number: Some(8),
                tags: Some(vec!["test".to_string()]),
                description: Some("test\ndesc".to_string()),
                ..Default::default()
            },
            Task {
                name: "test 3".to_string(),
                line_number: Some(8),
                tags: Some(vec!["test".to_string()]),
                description: Some("test\ndesc".to_string()),
                ..Default::default()
            },
        ];
        let res = filter_tasks_to_vec(
            &input,
            &Filter {
                task: Task {
                    name: String::from("target"),
                    tags: Some(vec!["test".to_string()]),
                    due_date: Some(Date::Day(NaiveDate::from_ymd_opt(2020, 2, 2).unwrap())),
                    ..Default::default()
                },
                inverted: true,
                state: None,
            },
        );
        assert_eq!(res, expected);
    }

    #[test]
    fn filter_subtasks_test() {
        // Create a vault structure with a task that has a subtask
        // The filter should match the parent task because it contains a matching subtask
        let input = Vaults::new(vec![VaultNode::Directory {
            name: "test".to_owned(),
            path: PathBuf::from("test"),
            content: vec![VaultNode::File {
                name: "Test".to_string(),
                path: PathBuf::from("test/Test.md"),
                content: vec![
                    FileEntryNode::Header {
                        name: "1".to_string(),
                        heading_level: 1,
                        content: vec![FileEntryNode::Header {
                            name: "2".to_string(),
                            heading_level: 2,
                            content: vec![FileEntryNode::Task(Task {
                                name: "task".to_string(),
                                line_number: Some(8),
                                tags: Some(vec!["test".to_string()]),
                                description: Some("test\ndesc".to_string()),
                                subtasks: vec![Task {
                                    name: "subtask".to_string(),
                                    ..Default::default()
                                }],
                                ..Default::default()
                            })],
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
                                content: vec![FileEntryNode::Task(Task {
                                    name: "false target 2".to_string(),
                                    line_number: Some(8),
                                    tags: Some(vec!["test".to_string()]),
                                    description: Some("test\ndesc".to_string()),
                                    ..Default::default()
                                })],
                            },
                        ],
                    },
                    FileEntryNode::Task(Task {
                        name: "test 3".to_string(),
                        line_number: Some(8),
                        tags: Some(vec!["test".to_string()]),
                        description: Some("test\ndesc".to_string()),
                        ..Default::default()
                    }),
                ],
            }],
        }]);

        // Expected result: only the structure containing the matching subtask
        let expected = Some(Vaults::new(vec![VaultNode::Directory {
            name: "test".to_owned(),
            path: PathBuf::from("test"),
            content: vec![VaultNode::File {
                name: "Test".to_string(),
                path: PathBuf::from("test/Test.md"),
                content: vec![FileEntryNode::Header {
                    name: "1".to_string(),
                    heading_level: 1,
                    content: vec![FileEntryNode::Header {
                        name: "2".to_string(),
                        heading_level: 2,
                        content: vec![FileEntryNode::Task(Task {
                            name: "task".to_string(),
                            line_number: Some(8),
                            tags: Some(vec!["test".to_string()]),
                            description: Some("test\ndesc".to_string()),
                            subtasks: vec![Task {
                                name: "subtask".to_string(),
                                ..Default::default()
                            }],
                            ..Default::default()
                        })],
                    }],
                }],
            }],
        }]));

        let res = filter(
            &input,
            &Some(Filter {
                task: Task {
                    name: String::from("subtask"),
                    ..Default::default()
                },
                inverted: false,
                state: None,
            }),
        );
        assert_eq!(res, expected);
    }

    #[test]
    fn filter_priority_test() {
        let input = Vaults::new(vec![VaultNode::Directory {
            name: "test".to_owned(),
            path: PathBuf::from("test"),
            content: vec![VaultNode::File {
                name: "untitled.md".to_string(),
                path: PathBuf::from("test/untitled.md"),
                content: vec![
                    FileEntryNode::Task(Task {
                        name: "high priority task".to_string(),
                        priority: 5,
                        ..Default::default()
                    }),
                    FileEntryNode::Task(Task {
                        name: "normal priority task".to_string(),
                        priority: 0,
                        ..Default::default()
                    }),
                    FileEntryNode::Task(Task {
                        name: "medium priority task".to_string(),
                        priority: 3,
                        ..Default::default()
                    }),
                ],
            }],
        }]);
        let expected = vec![Task {
            name: "high priority task".to_string(),
            priority: 5,
            ..Default::default()
        }];
        let res = filter_tasks_to_vec(
            &input,
            &Filter {
                task: Task {
                    priority: 5,
                    ..Default::default()
                },
                inverted: false,
                state: None,
            },
        );
        assert_eq!(res, expected);
    }

    #[test]
    fn filter_priority_inverted_test() {
        let input = Vaults::new(vec![VaultNode::Directory {
            name: "test".to_owned(),
            path: PathBuf::from("test"),
            content: vec![VaultNode::File {
                name: "untitled.md".to_string(),
                path: PathBuf::from("test/untitled.md"),
                content: vec![
                    FileEntryNode::Task(Task {
                        name: "high priority task".to_string(),
                        priority: 5,
                        ..Default::default()
                    }),
                    FileEntryNode::Task(Task {
                        name: "normal priority task".to_string(),
                        priority: 0,
                        ..Default::default()
                    }),
                    FileEntryNode::Task(Task {
                        name: "medium priority task".to_string(),
                        priority: 3,
                        ..Default::default()
                    }),
                ],
            }],
        }]);
        let expected = vec![
            Task {
                name: "normal priority task".to_string(),
                priority: 0,
                ..Default::default()
            },
            Task {
                name: "medium priority task".to_string(),
                priority: 3,
                ..Default::default()
            },
        ];
        let res = filter_tasks_to_vec(
            &input,
            &Filter {
                task: Task {
                    priority: 5,
                    ..Default::default()
                },
                inverted: true,
                state: None,
            },
        );
        assert_eq!(res, expected);
    }

    #[test]
    fn filter_state_test() {
        let input = Vaults::new(vec![VaultNode::Directory {
            name: "test".to_owned(),
            path: PathBuf::from("test"),
            content: vec![VaultNode::File {
                name: "untitled.md".to_string(),
                path: PathBuf::from("test/untitled.md"),
                content: vec![
                    FileEntryNode::Task(Task {
                        name: "todo task".to_string(),
                        state: State::ToDo,
                        ..Default::default()
                    }),
                    FileEntryNode::Task(Task {
                        name: "done task".to_string(),
                        state: State::Done,
                        ..Default::default()
                    }),
                    FileEntryNode::Task(Task {
                        name: "incomplete task".to_string(),
                        state: State::Incomplete,
                        ..Default::default()
                    }),
                ],
            }],
        }]);
        let expected = vec![
            Task {
                name: "todo task".to_string(),
                state: State::ToDo,
                ..Default::default()
            },
            Task {
                name: "incomplete task".to_string(),
                state: State::Incomplete,
                ..Default::default()
            },
        ];
        let res = filter_tasks_to_vec(
            &input,
            &Filter {
                task: Task {
                    state: State::ToDo,
                    ..Default::default()
                },
                inverted: false,
                state: Some(State::ToDo),
            },
        );
        assert_eq!(res, expected);
    }

    #[test]
    fn filter_state_inverted_test() {
        let input = Vaults::new(vec![VaultNode::Directory {
            name: "test".to_owned(),
            path: PathBuf::from("test"),
            content: vec![VaultNode::File {
                name: "untitled.md".to_string(),
                path: PathBuf::from("test/untitled.md"),
                content: vec![
                    FileEntryNode::Task(Task {
                        name: "todo task".to_string(),
                        state: State::ToDo,
                        ..Default::default()
                    }),
                    FileEntryNode::Task(Task {
                        name: "done task".to_string(),
                        state: State::Done,
                        ..Default::default()
                    }),
                    FileEntryNode::Task(Task {
                        name: "incomplete task".to_string(),
                        state: State::Incomplete,
                        ..Default::default()
                    }),
                ],
            }],
        }]);
        let expected = vec![Task {
            name: "done task".to_string(),
            state: State::Done,
            ..Default::default()
        }];
        let res = filter_tasks_to_vec(
            &input,
            &Filter {
                task: Task {
                    state: State::ToDo,
                    ..Default::default()
                },
                inverted: true,
                state: Some(State::ToDo),
            },
        );
        assert_eq!(res, expected);
    }

    #[test]
    fn filter_today_flag_test() {
        let input = Vaults::new(vec![VaultNode::Directory {
            name: "test".to_owned(),
            path: PathBuf::from("test"),
            content: vec![VaultNode::File {
                name: "untitled.md".to_string(),
                path: PathBuf::from("test/untitled.md"),
                content: vec![
                    FileEntryNode::Task(Task {
                        name: "today task".to_string(),
                        is_today: true,
                        ..Default::default()
                    }),
                    FileEntryNode::Task(Task {
                        name: "normal task".to_string(),
                        is_today: false,
                        ..Default::default()
                    }),
                    FileEntryNode::Task(Task {
                        name: "another normal task".to_string(),
                        is_today: false,
                        ..Default::default()
                    }),
                ],
            }],
        }]);
        let expected = vec![Task {
            name: "today task".to_string(),
            is_today: true,
            ..Default::default()
        }];
        let res = filter_tasks_to_vec(
            &input,
            &Filter {
                task: Task {
                    is_today: true,
                    ..Default::default()
                },
                inverted: false,
                state: None,
            },
        );
        assert_eq!(res, expected);
    }

    #[test]
    fn filter_today_flag_inverted_test() {
        let input = Vaults::new(vec![VaultNode::Directory {
            name: "test".to_owned(),
            path: PathBuf::from("test"),
            content: vec![VaultNode::File {
                name: "untitled.md".to_string(),
                path: PathBuf::from("test/untitled.md"),
                content: vec![
                    FileEntryNode::Task(Task {
                        name: "today task".to_string(),
                        is_today: true,
                        ..Default::default()
                    }),
                    FileEntryNode::Task(Task {
                        name: "normal task".to_string(),
                        is_today: false,
                        ..Default::default()
                    }),
                    FileEntryNode::Task(Task {
                        name: "another normal task".to_string(),
                        is_today: false,
                        ..Default::default()
                    }),
                ],
            }],
        }]);
        let expected = vec![
            Task {
                name: "normal task".to_string(),
                is_today: false,
                ..Default::default()
            },
            Task {
                name: "another normal task".to_string(),
                is_today: false,
                ..Default::default()
            },
        ];
        let res = filter_tasks_to_vec(
            &input,
            &Filter {
                task: Task {
                    is_today: true,
                    ..Default::default()
                },
                inverted: true,
                state: None,
            },
        );
        assert_eq!(res, expected);
    }
}
