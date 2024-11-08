use crate::{config::Config, task_core::task::DueDate};

use super::{
    parser::task::parse_task,
    task::{State, Task},
    vault_data::VaultData,
};

#[derive(PartialEq, Eq, Debug)]
pub struct Filter {
    pub task: Task,
    state: Option<State>,
}

/// Parses a [`Task`] from an input `&str`. Returns the `Task` and whether the input specify a task state (- [X] or - [ ]) or not.
pub fn parse_search_input(input: &str, config: &Config) -> Filter {
    // Are searching for a specific state ?
    let has_state = input.starts_with("- [");

    // Make the input parsable, add a task state if needed
    let input_value = format!("{}{}", if has_state { "" } else { "- [ ]" }, input);

    // Parse the input
    let task = match parse_task(&mut input_value.as_str(), String::new(), config) {
        Ok(t) => t,
        Err(_e) => Task {
            name: String::from("Uncomplete search prompt"),
            ..Default::default()
        },
    };
    Filter {
        task: task.clone(),
        state: if has_state { Some(task.state) } else { None },
    }
}

fn filter_task(task: &Task, filter: &Filter) -> bool {
    let state_match = filter.state.is_none()
        || filter
            .state
            .clone()
            .is_some_and(|state| state == task.state);

    let name_match = if filter.task.name.is_empty() {
        true
    } else {
        // for each word of the filter_task, if at least one
        // matches in the task, then validate
        filter
            .task
            .name
            .to_lowercase()
            .split_whitespace()
            .filter(|w| task.name.to_lowercase().contains(w))
            .count()
            > 0
    };

    let today_flag_match = if filter.task.is_today {
        task.is_today
    } else {
        true
    };

    let date_match = match (task.due_date.clone(), filter.task.due_date.clone()) {
        (_, DueDate::NoDate) => true,
        (DueDate::DayTime(task_date), DueDate::DayTime(search_date))
            if task_date == search_date =>
        {
            true
        }
        (DueDate::Day(task_date), DueDate::Day(search_date)) if task_date == search_date => true,
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

    state_match && name_match && today_flag_match && date_match && tags_match && priority_match
}
pub fn filter_to_vec(vault_data: &VaultData, filter: &Filter) -> Vec<Task> {
    fn aux(vault_data: &VaultData, task_filter: &Filter, res: &mut Vec<Task>) {
        match vault_data {
            VaultData::Directory(_, children) | VaultData::Header(_, _, children) => {
                for c in children {
                    aux(&c.clone(), task_filter, res);
                }
            }
            VaultData::Task(task) => {
                if !filter_task(task, task_filter) {
                    return;
                }

                res.push(task.clone());
                task.subtasks
                    .iter()
                    .for_each(|t| aux(&VaultData::Task(t.clone()), task_filter, res));
            }
        }
    }
    let res = &mut vec![];
    aux(vault_data, filter, res);
    res.clone()
}
pub fn filter(vault_data: &VaultData, task_filter: &Filter) -> Option<VaultData> {
    match vault_data {
        VaultData::Header(level, name, children) => {
            let mut actual_children = vec![];
            for child in children {
                let child_clone = child.clone();
                if let Some(child) = filter(&child_clone, task_filter) {
                    actual_children.push(child);
                }
            }
            if actual_children.is_empty() {
                None
            } else {
                Some(VaultData::Header(*level, name.to_string(), actual_children))
            }
        }
        VaultData::Directory(name, children) => {
            let mut actual_children = vec![];
            for child in children {
                let child_clone = child.clone();
                if let Some(child) = filter(&child_clone, task_filter) {
                    actual_children.push(child);
                }
            }
            if actual_children.is_empty() {
                None
            } else {
                Some(VaultData::Directory(name.to_string(), actual_children))
            }
        }
        VaultData::Task(task) => {
            if filter_task(task, task_filter) {
                Some(vault_data.clone())
            } else {
                let mut actual_children = vec![];
                for child in &task.subtasks {
                    if let Some(VaultData::Task(child)) =
                        filter(&VaultData::Task(child.clone()), task_filter)
                    {
                        actual_children.push(child);
                    }
                }
                if actual_children.is_empty() {
                    return None;
                }
                Some(VaultData::Task(Task {
                    subtasks: actual_children,
                    ..task.clone()
                }))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use crate::{
        config::Config,
        task_core::{
            filter::{filter, Filter},
            task::{DueDate, State, Task},
            vault_data::VaultData,
        },
    };

    use super::{filter_to_vec, parse_search_input};

    #[test]
    fn parse_search_input_test() {
        let input = "- [ ] #tag today name p5";
        let config = Config::default();
        let res = parse_search_input(input, &config);
        let expected = Filter {
            task: Task {
                due_date: DueDate::Day(chrono::Local::now().date_naive()),
                name: String::from("name"),
                priority: 5,
                state: State::ToDo,
                tags: Some(vec![String::from("tag")]),
                ..Default::default()
            },
            state: Some(State::ToDo),
        };
        assert_eq!(expected, res);
    }

    #[test]
    fn parse_search_input_no_state_test() {
        let input = "#tag today name p5";
        let config = Config::default();
        let res = parse_search_input(input, &config);
        let expected = Filter {
            task: Task {
                due_date: DueDate::Day(chrono::Local::now().date_naive()),
                name: String::from("name"),
                priority: 5,
                state: State::ToDo,
                tags: Some(vec![String::from("tag")]),
                ..Default::default()
            },
            state: None,
        };
        assert_eq!(expected, res);
    }

    #[test]
    fn filter_tags_test() {
        let input = VaultData::Directory(
            "test".to_owned(),
            vec![
                VaultData::Header(
                    0,
                    "Test".to_string(),
                    vec![
                        VaultData::Header(
                            1,
                            "1".to_string(),
                            vec![VaultData::Header(
                                2,
                                "2".to_string(),
                                vec![VaultData::Task(Task {
                                    name: "test 1".to_string(),
                                    line_number: 8,
                                    description: Some("test\ndesc".to_string()),
                                    ..Default::default()
                                })],
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
                                        name: "test 2".to_string(),
                                        line_number: 8,
                                        tags: Some(vec!["test".to_string()]),
                                        description: Some("test\ndesc".to_string()),
                                        ..Default::default()
                                    })],
                                ),
                            ],
                        ),
                    ],
                ),
                VaultData::Task(Task {
                    name: "test 3".to_string(),
                    line_number: 8,
                    tags: Some(vec!["test".to_string()]),
                    description: Some("test\ndesc".to_string()),
                    ..Default::default()
                }),
            ],
        );
        let expected = vec![
            Task {
                name: "test 2".to_string(),
                line_number: 8,
                tags: Some(vec!["test".to_string()]),
                description: Some("test\ndesc".to_string()),
                ..Default::default()
            },
            Task {
                name: "test 3".to_string(),
                line_number: 8,
                tags: Some(vec!["test".to_string()]),
                description: Some("test\ndesc".to_string()),
                ..Default::default()
            },
        ];
        let res = filter_to_vec(
            &input,
            &Filter {
                task: Task {
                    name: String::new(),
                    tags: Some(vec!["test".to_string()]),
                    ..Default::default()
                },
                state: None,
            },
        );
        assert_eq!(res, expected);
    }
    #[test]
    fn filter_names_test() {
        let input = VaultData::Directory(
            "test".to_owned(),
            vec![
                VaultData::Header(
                    0,
                    "Test".to_string(),
                    vec![
                        VaultData::Header(
                            1,
                            "1".to_string(),
                            vec![VaultData::Header(
                                2,
                                "2".to_string(),
                                vec![VaultData::Task(Task {
                                    name: "hfdgqskhjfg1".to_string(),
                                    line_number: 8,
                                    description: Some("test\ndesc".to_string()),
                                    ..Default::default()
                                })],
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
                                        name: "test 2".to_string(),
                                        line_number: 8,
                                        tags: Some(vec!["test".to_string()]),
                                        description: Some("test\ndesc".to_string()),
                                        ..Default::default()
                                    })],
                                ),
                            ],
                        ),
                    ],
                ),
                VaultData::Task(Task {
                    name: "test 3".to_string(),
                    line_number: 8,
                    tags: Some(vec!["test".to_string()]),
                    description: Some("test\ndesc".to_string()),
                    ..Default::default()
                }),
            ],
        );
        let expected = vec![
            Task {
                name: "test 2".to_string(),
                line_number: 8,
                tags: Some(vec!["test".to_string()]),
                description: Some("test\ndesc".to_string()),
                ..Default::default()
            },
            Task {
                name: "test 3".to_string(),
                line_number: 8,
                tags: Some(vec!["test".to_string()]),
                description: Some("test\ndesc".to_string()),
                ..Default::default()
            },
        ];
        let res = filter_to_vec(
            &input,
            &Filter {
                task: Task {
                    name: String::from("test"),
                    ..Default::default()
                },
                state: None,
            },
        );
        assert_eq!(res, expected);
    }
    #[test]
    fn filter_due_date_test() {
        let input = VaultData::Directory(
            "test".to_owned(),
            vec![
                VaultData::Header(
                    0,
                    "Test".to_string(),
                    vec![
                        VaultData::Header(
                            1,
                            "1".to_string(),
                            vec![VaultData::Header(
                                2,
                                "2".to_string(),
                                vec![VaultData::Task(Task {
                                    name: "hfdgqskhjfg1".to_string(),
                                    line_number: 8,
                                    due_date: DueDate::Day(
                                        NaiveDate::from_ymd_opt(2020, 2, 2).unwrap(),
                                    ),
                                    description: Some("test\ndesc".to_string()),
                                    ..Default::default()
                                })],
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
                                        name: "test 2".to_string(),
                                        line_number: 8,
                                        tags: Some(vec!["test".to_string()]),
                                        description: Some("test\ndesc".to_string()),
                                        ..Default::default()
                                    })],
                                ),
                            ],
                        ),
                    ],
                ),
                VaultData::Task(Task {
                    name: "test 3".to_string(),
                    line_number: 8,
                    tags: Some(vec!["test".to_string()]),
                    description: Some("test\ndesc".to_string()),
                    ..Default::default()
                }),
            ],
        );
        let expected = vec![Task {
            name: "hfdgqskhjfg1".to_string(),
            line_number: 8,
            due_date: DueDate::Day(NaiveDate::from_ymd_opt(2020, 2, 2).unwrap()),
            description: Some("test\ndesc".to_string()),
            ..Default::default()
        }];
        let res = filter_to_vec(
            &input,
            &Filter {
                task: Task {
                    due_date: DueDate::Day(NaiveDate::from_ymd_opt(2020, 2, 2).unwrap()),
                    ..Default::default()
                },
                state: None,
            },
        );
        assert_eq!(res, expected);
    }
    #[test]
    fn filter_full_test() {
        let input = VaultData::Directory(
            "test".to_owned(),
            vec![
                VaultData::Header(
                    0,
                    "Test".to_string(),
                    vec![
                        VaultData::Header(
                            1,
                            "1".to_string(),
                            vec![VaultData::Header(
                                2,
                                "2".to_string(),
                                vec![VaultData::Task(Task {
                                    name: "real target".to_string(),
                                    line_number: 8,
                                    tags: Some(vec!["test".to_string()]),
                                    due_date: DueDate::Day(
                                        NaiveDate::from_ymd_opt(2020, 2, 2).unwrap(),
                                    ),
                                    description: Some("test\ndesc".to_string()),
                                    ..Default::default()
                                })],
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
                                        name: "false target 2".to_string(),
                                        line_number: 8,
                                        tags: Some(vec!["test".to_string()]),
                                        description: Some("test\ndesc".to_string()),
                                        ..Default::default()
                                    })],
                                ),
                            ],
                        ),
                    ],
                ),
                VaultData::Task(Task {
                    name: "test 3".to_string(),
                    line_number: 8,
                    tags: Some(vec!["test".to_string()]),
                    description: Some("test\ndesc".to_string()),
                    ..Default::default()
                }),
            ],
        );
        let expected = vec![Task {
            name: "real target".to_string(),
            line_number: 8,
            due_date: DueDate::Day(NaiveDate::from_ymd_opt(2020, 2, 2).unwrap()),
            tags: Some(vec!["test".to_string()]),
            description: Some("test\ndesc".to_string()),
            ..Default::default()
        }];
        let res = filter_to_vec(
            &input,
            &Filter {
                task: Task {
                    name: String::from("target"),
                    tags: Some(vec!["test".to_string()]),
                    due_date: DueDate::Day(NaiveDate::from_ymd_opt(2020, 2, 2).unwrap()),
                    ..Default::default()
                },
                state: None,
            },
        );
        assert_eq!(res, expected);
    }
    #[test]
    fn filter_subtasks_test() {
        let input = VaultData::Directory(
            "test".to_owned(),
            vec![
                VaultData::Header(
                    0,
                    "Test".to_string(),
                    vec![
                        VaultData::Header(
                            1,
                            "1".to_string(),
                            vec![VaultData::Header(
                                2,
                                "2".to_string(),
                                vec![VaultData::Task(Task {
                                    name: "task".to_string(),
                                    line_number: 8,
                                    tags: Some(vec!["test".to_string()]),
                                    description: Some("test\ndesc".to_string()),
                                    subtasks: vec![Task {
                                        name: "subtask".to_string(),
                                        ..Default::default()
                                    }],
                                    ..Default::default()
                                })],
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
                                        name: "false target 2".to_string(),
                                        line_number: 8,
                                        tags: Some(vec!["test".to_string()]),
                                        description: Some("test\ndesc".to_string()),
                                        ..Default::default()
                                    })],
                                ),
                            ],
                        ),
                    ],
                ),
                VaultData::Task(Task {
                    name: "test 3".to_string(),
                    line_number: 8,
                    tags: Some(vec!["test".to_string()]),
                    description: Some("test\ndesc".to_string()),
                    ..Default::default()
                }),
            ],
        );
        let expected = Some(VaultData::Directory(
            "test".to_owned(),
            vec![VaultData::Header(
                0,
                "Test".to_string(),
                vec![VaultData::Header(
                    1,
                    "1".to_string(),
                    vec![VaultData::Header(
                        2,
                        "2".to_string(),
                        vec![VaultData::Task(Task {
                            name: "task".to_string(),
                            line_number: 8,
                            tags: Some(vec!["test".to_string()]),
                            description: Some("test\ndesc".to_string()),
                            subtasks: vec![Task {
                                name: "subtask".to_string(),
                                ..Default::default()
                            }],
                            ..Default::default()
                        })],
                    )],
                )],
            )],
        ));
        let res = filter(
            &input,
            &Filter {
                task: Task {
                    name: String::from("subtask"),
                    ..Default::default()
                },
                state: None,
            },
        );
        assert_eq!(res, expected);
    }
}
