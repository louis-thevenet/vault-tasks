use ratatui::prelude::*;
use tui_scrollview::{ScrollView, ScrollViewState};
use vault_tasks_core::vault_data::VaultData;

use crate::config::Config;

use super::task_list_item::TaskListItem;

#[derive(Default, Clone)]
pub struct TaskList {
    file_content: Vec<VaultData>,
    not_american_format: bool,
    show_relative_due_dates: bool,
    display_filename: bool,
    header_style: Style,
}

impl TaskList {
    pub fn new(config: &Config, file_content: &[VaultData], display_filename: bool) -> Self {
        Self {
            not_american_format: !config.tasks_config.use_american_format,
            file_content: file_content.to_vec(),
            display_filename,
            header_style: Style::default(),
            show_relative_due_dates: config.tasks_config.show_relative_due_dates,
        }
    }
    pub const fn header_style(mut self, style: Style) -> Self {
        self.header_style = style;
        self
    }
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
        let content = self
            .file_content
            .iter()
            .map(|fc| {
                TaskListItem::new(
                    fc.clone(),
                    self.not_american_format,
                    self.display_filename,
                    self.show_relative_due_dates,
                )
                .header_style(self.header_style)
            })
            .collect::<Vec<TaskListItem>>();

        let mut constraints = vec![];
        let mut height = 0;
        for item in &content {
            height += item.height;
            constraints.push(Constraint::Length(item.height));
        }

        // If we need the vertical scrollbar
        // Then take into account that we need to draw it
        //
        // If we don't do this, the horizontal scrollbar
        // appears for only one character
        // It basically disables the horizontal scrollbar
        let width = if height > area.height {
            area.width - 1
        } else {
            area.width
        };

        let size = Size::new(width, height);
        let mut scroll_view = ScrollView::new(size);

        let layout = Layout::vertical(constraints).split(scroll_view.area());

        for (i, item) in content.into_iter().enumerate() {
            scroll_view.render_widget(item, layout[i]);
        }
        scroll_view.render(area, buf, state);
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use insta::assert_snapshot;
    use ratatui::{backend::TestBackend, Terminal};
    use tui_scrollview::ScrollViewState;
    use vault_tasks_core::{
        task::{DueDate, State, Task},
        vault_data::VaultData,
    };

    use crate::{config::Config, widgets::task_list::TaskList};

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

        let mut config = Config::default();

        // We don't want tests to be time dependent
        config.tasks_config.show_relative_due_dates = false;

        let task_list = TaskList::new(&config, &[test_vault], true);
        let mut terminal = Terminal::new(TestBackend::new(40, 40)).unwrap();
        terminal
            .draw(|frame| {
                frame.render_stateful_widget(task_list, frame.area(), &mut ScrollViewState::new());
            })
            .unwrap();
        assert_snapshot!(terminal.backend());
    }
}
