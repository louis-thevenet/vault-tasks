use tracing::debug;

use crate::{config::Config, task_core::task::DueDate};

use super::{parser::task::parse_task, task::Task, vault_data::VaultData};

pub fn parse_search_input(input: &str, config: &Config) -> (Task, bool) {
    // Are searching for a specific state ?
    let has_state = input.starts_with("- [");

    // Make the input parsable
    let input_value = format!("{}{}", if has_state { "" } else { "- [ ]" }, input);

    // Parse the input
    let search = match parse_task(&mut input_value.as_str(), config) {
        Ok(t) => t,
        Err(_e) => Task {
            name: String::from("Uncomplete search prompt"),
            ..Default::default()
        },
    };
    (search, has_state)
}

fn tasks_match(to_search: &Task, to_filter: &Task, compare_states: bool) -> bool {
    let state_match = to_search.state == to_filter.state;

    let name_match = if to_search.name.is_empty() {
        true
    } else {
        to_filter
            .name
            .to_lowercase()
            .contains(&to_search.name.to_lowercase())
    };

    let date_match = match (to_filter.due_date.clone(), to_search.due_date.clone()) {
        (_, DueDate::NoDate) => true,
        (DueDate::DayTime(task_date), DueDate::DayTime(search_date))
            if task_date == search_date =>
        {
            true
        }
        (DueDate::Day(task_date), DueDate::Day(search_date)) if task_date == search_date => true,
        (_, _) => false,
    };

    let tags_match = to_search.tags.clone().unwrap_or_default().iter().all(|t| {
        to_filter
            .tags
            .clone()
            .unwrap_or_default()
            .iter()
            .any(|x| x.to_lowercase().contains(&t.to_lowercase()))
    });

    let priority_match = if to_search.priority > 0 {
        to_search.priority == to_filter.priority
    } else {
        true
    };

    (!compare_states || state_match) && name_match && date_match && tags_match && priority_match
}

pub fn filter_to_vec(vault_data: &VaultData, search: &Task, compare_states: bool) -> Vec<Task> {
    fn aux(vault_data: &VaultData, search: &Task, compare_states: bool, res: &mut Vec<Task>) {
        match vault_data {
            VaultData::Directory(_, children) | VaultData::Header(_, _, children) => {
                for c in children {
                    aux(&c.clone(), search, compare_states, res);
                }
            }
            VaultData::Task(task) => {
                if tasks_match(task, search, compare_states) {
                    res.push(task.clone());
                }

                task.subtasks
                    .iter()
                    .for_each(|t| aux(&VaultData::Task(t.clone()), search, compare_states, res));
            }
        }
    }
    let res = &mut vec![];
    aux(vault_data, search, compare_states, res);
    res.clone()
}
pub fn filter(vault_data: &VaultData, search: &Task, compare_states: bool) -> Option<VaultData> {
    match vault_data {
        VaultData::Header(level, name, children) => {
            let mut actual_children = vec![];
            for child in children {
                let mut child_clone = child.clone();
                if filter(&mut child_clone, search, compare_states).is_some() {
                    actual_children.push(child_clone);
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
                let mut child_clone = child.clone();
                if filter(&mut child_clone, search, compare_states).is_some() {
                    actual_children.push(child_clone);
                }
            }
            if actual_children.is_empty() {
                None
            } else {
                Some(VaultData::Directory(name.to_string(), actual_children))
            }
        }
        VaultData::Task(task) => {
            if tasks_match(search, task, compare_states) {
                Some(vault_data.clone())
            } else {
                let mut actual_children = vec![];
                for child in task.subtasks.iter() {
                    if filter(&mut VaultData::Task(child.clone()), search, compare_states).is_some()
                    {
                        actual_children.push(child.clone());
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

    use crate::task_core::{
        task::{DueDate, Task},
        vault_data::VaultData,
    };

    use super::filter_to_vec;

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
            &Task {
                name: String::new(),
                tags: Some(vec!["test".to_string()]),
                ..Default::default()
            },
            false,
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
            &Task {
                name: String::from("test"),
                ..Default::default()
            },
            false,
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
            &Task {
                due_date: DueDate::Day(NaiveDate::from_ymd_opt(2020, 2, 2).unwrap()),
                ..Default::default()
            },
            false,
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
            &Task {
                name: String::from("target"),
                tags: Some(vec!["test".to_string()]),
                due_date: DueDate::Day(NaiveDate::from_ymd_opt(2020, 2, 2).unwrap()),
                ..Default::default()
            },
            false,
        );
        assert_eq!(res, expected);
    }
}
