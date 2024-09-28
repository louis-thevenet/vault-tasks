use crate::task_core::task::DueDate;

use super::{task::Task, vault_data::VaultData};

pub fn filter(vault_data: &VaultData, search: &Task) -> Vec<Task> {
    fn aux(vault_data: &VaultData, search: &Task, res: &mut Vec<Task>) {
        match vault_data {
            VaultData::Directory(_, children) | VaultData::Header(_, _, children) => {
                for c in children {
                    aux(&c.clone(), search, res);
                }
            }
            VaultData::Task(task) => {
                let name_match = if search.name.is_empty() {
                    true
                } else {
                    task.name.contains(&search.name)
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

                let tags_match = search
                    .tags
                    .clone()
                    .unwrap_or_default()
                    .iter()
                    .all(|t| task.tags.clone().unwrap_or_default().contains(t));

                let priority_match = if search.priority > 0 {
                    search.priority == task.priority
                } else {
                    true
                };

                let desc_match = search.description.as_ref().map_or(true, |desc_search| {
                    task.description
                        .as_ref()
                        .map_or(true, |desc_task| desc_task.contains(desc_search))
                });

                if name_match && date_match && tags_match && priority_match && desc_match {
                    res.push(task.clone());
                }

                task.subtasks
                    .iter()
                    .for_each(|t| aux(&VaultData::Task(t.clone()), search, res));
            }
        }
    }
    let res = &mut vec![];
    aux(vault_data, search, res);
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
        );
        assert_eq!(res, expected);
    }
    fn filter_desc_test() {
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
                    ..Default::default()
                }),
            ],
        );
        let expected = vec![
            Task {
                name: "test 1".to_string(),
                line_number: 8,
                description: Some("test\ndesc".to_string()),
                ..Default::default()
            },
            Task {
                name: "test 2".to_string(),
                line_number: 8,
                tags: Some(vec!["test".to_string()]),
                description: Some("test\ndesc".to_string()),
                ..Default::default()
            },
        ];
        let res = filter(
            &input,
            &Task {
                name: String::from("desc"),
                ..Default::default()
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
        let res = filter(
            &input,
            &Task {
                name: String::from("test"),
                ..Default::default()
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
        let res = filter(
            &input,
            &Task {
                due_date: DueDate::Day(NaiveDate::from_ymd_opt(2020, 2, 2).unwrap()),
                ..Default::default()
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
        let res = filter(
            &input,
            &Task {
                name: String::from("target"),
                tags: Some(vec!["test".to_string()]),
                due_date: DueDate::Day(NaiveDate::from_ymd_opt(2020, 2, 2).unwrap()),
                ..Default::default()
            },
        );
        assert_eq!(res, expected);
    }
}
