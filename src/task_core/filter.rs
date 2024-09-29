use crate::task_core::task::DueDate;

use super::{task::Task, vault_data::VaultData};

pub fn filter(vault_data: &VaultData, search: &Task, compare_states: bool) -> Vec<Task> {
    fn aux(vault_data: &VaultData, search: &Task, compare_states: bool, res: &mut Vec<Task>) {
        match vault_data {
            VaultData::Directory(_, children) | VaultData::Header(_, _, children) => {
                for c in children {
                    aux(&c.clone(), search, compare_states, res);
                }
            }
            VaultData::Task(task) => {
                let state_match = search.state == task.state;

                let name_match = if search.name.is_empty() {
                    true
                } else {
                    task.name
                        .to_lowercase()
                        .contains(&search.name.to_lowercase())
                };

                let date_match = match (task.due_date.clone(), search.due_date.clone()) {
                    (_, DueDate::NoDate) => true,
                    (DueDate::DayTime(task_date), DueDate::DayTime(search_date))
                        if task_date == search_date =>
                    {
                        true
                    }
                    (DueDate::Day(task_date), DueDate::Day(search_date))
                        if task_date == search_date =>
                    {
                        true
                    }
                    (_, _) => false,
                };

                let tags_match = search.tags.clone().unwrap_or_default().iter().all(|t| {
                    task.tags
                        .clone()
                        .unwrap_or_default()
                        .iter()
                        .any(|x| x.to_lowercase().contains(&t.to_lowercase()))
                });

                let priority_match = if search.priority > 0 {
                    search.priority == task.priority
                } else {
                    true
                };

                if (!compare_states || state_match)
                    && name_match
                    && date_match
                    && tags_match
                    && priority_match
                {
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

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use crate::task_core::{
        task::{DueDate, Task},
        vault_data::VaultData,
    };

    use super::filter;

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
        let res = filter(
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
        let res = filter(
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
        let res = filter(
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
        let res = filter(
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
