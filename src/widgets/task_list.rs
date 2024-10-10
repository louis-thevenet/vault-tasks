use ratatui::{
    prelude::*,
    widgets::{Block, Borders},
};

use tui_widget_list::{ListBuilder, ListState, ListView};

use crate::{config::Config, task_core::vault_data::VaultData};

use super::task_list_item::TaskListItem;

#[derive(Default, Clone)]
pub struct TaskList {
    file_content: Vec<VaultData>,
    not_american_format: bool,
    state: ListState,
    display_filename: bool,
}

impl TaskList {
    pub fn new(config: &Config, file_content: &[VaultData], display_filename: bool) -> Self {
        Self {
            state: ListState::default(),
            not_american_format: !config.tasks_config.use_american_format,
            file_content: file_content.to_vec(),
            display_filename,
        }
    }
}
impl Widget for TaskList {
    fn render(mut self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let surrounding_block = Block::default().borders(Borders::NONE);
        let count = self.file_content.len();

        let builder = ListBuilder::new(move |context| {
            let item = TaskListItem::new(
                self.file_content[context.index].clone(),
                self.not_american_format,
                self.display_filename,
            );
            let height = item.height;
            (item, height)
        });

        let lateral_entries_list = ListView::new(builder, count).block(surrounding_block);
        let state = &mut self.state;
        lateral_entries_list.render(area, buf, state);
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use insta::assert_snapshot;
    use ratatui::{backend::TestBackend, Terminal};

    use crate::{
        config::Config,
        task_core::{
            task::{DueDate, State, Task},
            vault_data::VaultData,
        },
        widgets::task_list::TaskList,
    };

    #[test]
    fn test_render_search_bar() {
        let test_vault = VaultData::Header(
            0,
            "Test".to_string(),
            vec![
                VaultData::Header(
                    1,
                    "1".to_string(),
                    vec![
                        VaultData::Task(Task {
                            name: "task 1".to_string(),
                            state: State::Done,
                            tags: Some(vec![String::from("tag"), String::from("tag2")]),
                            priority: 5,
                            due_date: DueDate::DayTime(
                                NaiveDate::from_ymd_opt(2016, 7, 8)
                                    .unwrap()
                                    .and_hms_opt(9, 10, 11)
                                    .unwrap(),
                            ),
                            subtasks: vec![
                                Task {
                                    name: "subtask test with desc".to_string(),
                                    description: Some("test\ndesc".to_string()),
                                    ..Default::default()
                                },
                                Task {
                                    name: "subtask test with tags".to_string(),
                                    tags: Some(vec![String::from("tag"), String::from("tag2")]),
                                    ..Default::default()
                                },
                                Task {
                                    name: "subtask test".to_string(),
                                    ..Default::default()
                                },
                            ],
                            ..Default::default()
                        }),
                        VaultData::Header(
                            2,
                            "1.1".to_string(),
                            vec![VaultData::Header(
                                3,
                                "1.1.1".to_string(),
                                vec![VaultData::Task(Task {
                                    name: "test 1.1.1".to_string(),
                                    description: Some("test\ndesc\n🥃".to_string()),
                                    ..Default::default()
                                })],
                            )],
                        ),
                    ],
                ),
                VaultData::Header(
                    1,
                    "2".to_string(),
                    vec![
                        VaultData::Header(3, "2.1".to_string(), vec![]),
                        VaultData::Header(
                            2,
                            "2.2".to_string(),
                            vec![VaultData::Task(Task {
                                name: "test 2.2".to_string(),
                                description: Some("test\ndesc".to_string()),
                                subtasks: vec![Task {
                                    name: "subtask 2.2".to_string(),

                                    due_date: DueDate::DayTime(
                                        NaiveDate::from_ymd_opt(2016, 7, 8)
                                            .unwrap()
                                            .and_hms_opt(9, 10, 11)
                                            .unwrap(),
                                    ),
                                    description: Some("test\ndesc".to_string()),
                                    tags: Some(vec![String::from("tag"), String::from("tag2")]),
                                    ..Default::default()
                                }],
                                ..Default::default()
                            })],
                        ),
                    ],
                ),
            ],
        );
        let task_list = TaskList::new(&Config::default(), &[test_vault], true);
        let mut terminal = Terminal::new(TestBackend::new(40, 40)).unwrap();
        terminal
            .draw(|frame| frame.render_widget(task_list, frame.area()))
            .unwrap();
        assert_snapshot!(terminal.backend());
    }
}
