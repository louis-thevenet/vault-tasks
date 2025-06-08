use crate::core::vault_data::VaultData;
use ratatui::prelude::*;
use tui_scrollview::{ScrollView, ScrollViewState};

use crate::config::Config;

use super::task_list_item::TaskListItem;

#[derive(Default, Clone)]
pub struct TaskList {
    content: Vec<TaskListItem>,
    constraints: Vec<Constraint>,
    height: u16,
}

impl TaskList {
    pub fn new(
        config: &Config,
        file_content: &[VaultData],
        max_width: u16,
        display_filename: bool,
    ) -> Self {
        let content = file_content
            .iter()
            .map(|fc| {
                TaskListItem::new(
                    fc.clone(),
                    !config.tasks_config.use_american_format,
                    config.tasks_config.pretty_symbols.clone(),
                    max_width,
                    display_filename,
                    config.tasks_config.show_relative_due_dates,
                    config.tasks_config.completion_bar_length,
                )
                .header_style(
                    *config
                        .styles
                        .get(&crate::app::Mode::Explorer)
                        .unwrap()
                        .get("preview_headers")
                        .unwrap(),
                )
            })
            .collect::<Vec<TaskListItem>>();
        let mut height = 0;
        let mut constraints = vec![];
        for item in &content {
            height += item.height;
            constraints.push(Constraint::Length(item.height));
        }
        Self {
            content,
            constraints,
            height,
        }
    }
    // pub fn height_of(&mut self, i: usize) -> u16 {
    //     (0..i).map(|i| self.content[i].height).sum()
    // }
}
impl StatefulWidget for TaskList {
    type State = ScrollViewState;
    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) where
        Self: Sized,
    {
        // If we need the vertical scrollbar
        // Then take into account that we need to draw it
        //
        // If we don't do this, the horizontal scrollbar
        // appears for only one character
        // It basically disables the horizontal scrollbar
        let width = if self.height > area.height {
            area.width - 1
        } else {
            area.width
        };

        let size = Size::new(width, self.height);
        let mut scroll_view = ScrollView::new(size);

        let layout = Layout::vertical(self.constraints).split(scroll_view.area());

        for (i, item) in self.content.into_iter().enumerate() {
            scroll_view.render_widget(item, layout[i]);
        }
        scroll_view.render(area, buf, state);
    }
}

#[cfg(test)]
mod tests {
    use crate::core::{
        date::DueDate,
        task::{State, Task},
        vault_data::VaultData,
    };
    use chrono::NaiveDate;
    use insta::assert_snapshot;
    use ratatui::{Terminal, backend::TestBackend};
    use tui_scrollview::ScrollViewState;

    use crate::{config::Config, widgets::task_list::TaskList};

    #[test]
    fn test_task_list() {
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

        let mut config = Config::default();

        // We don't want tests to be time dependent
        config.tasks_config.show_relative_due_dates = false;

        let max_width = 40;
        let task_list = TaskList::new(&config, &[test_vault], max_width, true);
        let mut terminal = Terminal::new(TestBackend::new(max_width, 40)).unwrap();
        terminal
            .draw(|frame| {
                frame.render_stateful_widget(task_list, frame.area(), &mut ScrollViewState::new());
            })
            .unwrap();
        assert_snapshot!(terminal.backend());
    }
}
