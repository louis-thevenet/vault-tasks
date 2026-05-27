use ratatui::{
    prelude::*,
    widgets::{Clear, Scrollbar, ScrollbarState},
};
use vault_tasks_core::vault_data::FileEntryNode;

use crate::{config::Config, widgets::task_list_state::TaskListState};

use super::task_list_item::TaskListItem;

#[derive(Default, Clone)]
pub struct TaskList {
    content: Vec<TaskListItem>,
    item_tops: Vec<u16>,
    height: u16,
}

impl TaskList {
    pub fn new(
        config: &Config,
        file_content: &[FileEntryNode],
        max_width: u16,
        display_filename: bool,
    ) -> Self {
        let content = file_content
            .iter()
            .map(|fc| {
                TaskListItem::new(
                    fc.clone(),
                    config.core.clone(),
                    config.tui.settings.clone(),
                    max_width,
                    display_filename,
                )
                .header_style(
                    *config
                        .tui
                        .styles
                        .get(&crate::app::Mode::Explorer)
                        .unwrap()
                        .get("preview_headers")
                        .unwrap(),
                )
            })
            .collect::<Vec<TaskListItem>>();
        let mut height = 0;
        let mut item_tops = Vec::with_capacity(content.len());
        for item in &content {
            item_tops.push(height);
            height += item.height;
        }
        Self {
            content,
            item_tops,
            height,
        }
    }

    fn render_visible_slice(
        item_buffer: &Buffer,
        target_buffer: &mut Buffer,
        target_area: Rect,
        source_row_start: u16,
        row_count: u16,
        target_y: u16,
    ) {
        for row in 0..row_count {
            for col in 0..target_area.width {
                if let Some(source_cell) = item_buffer.cell((col, source_row_start + row))
                    && let Some(target_cell) =
                        target_buffer.cell_mut((target_area.x + col, target_y + row))
                {
                    *target_cell = source_cell.clone();
                }
            }
        }
    }
}
impl StatefulWidget for TaskList {
    type State = TaskListState;
    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) where
        Self: Sized,
    {
        if area.is_empty() {
            return;
        }
        Clear.render(area, buf);

        let show_scrollbar = self.height > area.height;
        let layout_with_scrollbar = Layout::horizontal([Constraint::Min(1), Constraint::Length(3)]);
        let area_with_scrollbar: [Rect; 2] = layout_with_scrollbar.areas(area);

        let items_area = if show_scrollbar {
            area_with_scrollbar[0]
        } else {
            area
        };

        state.update_bounds(self.height, items_area.height);

        if self.content.is_empty() || items_area.height == 0 {
            return;
        }

        let visible_start = state.offset();
        let visible_end = visible_start.saturating_add(items_area.height);
        let start_index = self
            .item_tops
            .partition_point(|top| *top <= visible_start)
            .saturating_sub(1);

        let TaskList {
            content,
            item_tops,
            height: _,
        } = self;

        // Scrollbar
        if show_scrollbar {
            let scrollbar_area = area_with_scrollbar[1];
            let mut scrollbar_state =
                ScrollbarState::new(self.height as usize).position(state.offset() as usize);
            Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight).render(
                scrollbar_area,
                buf,
                &mut scrollbar_state,
            );
        } // Tasklist
        for (index, item) in content.into_iter().enumerate().skip(start_index) {
            let item_top = item_tops[index];
            let item_height = item.height;
            if item_top >= visible_end {
                break;
            }

            let item_bottom = item_top.saturating_add(item_height);
            let visible_row_start = visible_start.saturating_sub(item_top);
            let visible_row_end = item_bottom.min(visible_end).saturating_sub(item_top);
            let visible_rows = visible_row_end.saturating_sub(visible_row_start);
            let target_y = items_area.y + item_top.saturating_sub(visible_start);

            if visible_row_start == 0 && visible_rows == item_height {
                item.render(
                    Rect::new(items_area.x, target_y, items_area.width, item_height),
                    buf,
                );
            } else {
                let item_area = Rect::new(0, 0, items_area.width, item_height);
                let mut item_buffer = Buffer::empty(item_area);
                item.render(item_area, &mut item_buffer);
                Self::render_visible_slice(
                    &item_buffer,
                    buf,
                    items_area,
                    visible_row_start,
                    visible_rows,
                    target_y,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use insta::assert_snapshot;
    use ratatui::{Terminal, backend::TestBackend};
    use vault_tasks_core::{
        date::Date,
        task::{State, Task},
        vault_data::FileEntryNode,
    };

    use crate::{
        config::Config,
        widgets::task_list::{TaskList, TaskListState},
    };

    #[test]
    fn test_task_list() {
        // Create file entries that would come from a markdown file
        let file_content = vec![
            FileEntryNode::Header {
                name: "1".to_string(),
                path: std::path::PathBuf::new(),
                heading_level: 1,
                content: vec![
                    FileEntryNode::Task(Task {
                        name: "task 1".to_string(),
                        state: State::Done,
                        tags: Some(vec![String::from("tag"), String::from("tag2")]),
                        priority: 5,
                        due_date: Some(Date::DayTime(
                            NaiveDate::from_ymd_opt(2016, 7, 8)
                                .unwrap()
                                .and_hms_opt(9, 10, 11)
                                .unwrap(),
                        )),
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
                    FileEntryNode::Header {
                        name: "1.1".to_string(),
                        path: std::path::PathBuf::new(),
                        heading_level: 2,
                        content: vec![FileEntryNode::Header {
                            name: "1.1.1".to_string(),
                            path: std::path::PathBuf::new(),
                            heading_level: 3,
                            content: vec![FileEntryNode::Task(Task {
                                name: "test 1.1.1".to_string(),
                                description: Some("test\ndesc\n🥃".to_string()),
                                ..Default::default()
                            })],
                        }],
                    },
                ],
            },
            FileEntryNode::Header {
                name: "2".to_string(),
                path: std::path::PathBuf::new(),
                heading_level: 1,
                content: vec![
                    FileEntryNode::Header {
                        name: "2.1".to_string(),
                        path: std::path::PathBuf::new(),
                        heading_level: 3,
                        content: vec![],
                    },
                    FileEntryNode::Header {
                        name: "2.2".to_string(),
                        path: std::path::PathBuf::new(),
                        heading_level: 2,
                        content: vec![FileEntryNode::Task(Task {
                            name: "test 2.2".to_string(),
                            description: Some("test\ndesc".to_string()),
                            subtasks: vec![Task {
                                name: "subtask 2.2".to_string(),
                                due_date: Some(Date::DayTime(
                                    NaiveDate::from_ymd_opt(2016, 7, 8)
                                        .unwrap()
                                        .and_hms_opt(9, 10, 11)
                                        .unwrap(),
                                )),
                                description: Some("test\ndesc".to_string()),
                                tags: Some(vec![String::from("tag"), String::from("tag2")]),
                                ..Default::default()
                            }],
                            ..Default::default()
                        })],
                    },
                ],
            },
        ];

        let mut config = Config::default();

        // We don't want tests to be time dependent
        config.core.display.show_relative_due_dates = false;

        let max_width = 40;
        let task_list = TaskList::new(&config, &file_content, max_width, true);
        let mut terminal = Terminal::new(TestBackend::new(max_width, 40)).unwrap();
        terminal
            .draw(|frame| {
                frame.render_stateful_widget(task_list, frame.area(), &mut TaskListState::new());
            })
            .unwrap();
        assert_snapshot!(terminal.backend());
    }
}
