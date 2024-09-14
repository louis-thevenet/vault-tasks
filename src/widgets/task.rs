use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

use crate::task_core::task::{State, Task};

pub struct TaskWidget {
    task: Task,
    not_american_format: bool,
}

impl TaskWidget {
    // pub fn new(config: Config, task: Task) -> Self {
    //     Self {
    //         task,
    //         not_american_format: !config.tasks_config.use_american_format,
    //     }
    // }
}
impl Widget for TaskWidget {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let state = if self.task.state == State::ToDo {
            "❌"
        } else {
            "✅"
        };
        let title = Span::styled(format!("{state} {}", self.task.name), Style::default());
        let surrounding_block = Block::default().borders(Borders::ALL).title(title.clone());

        let mut lines = vec![];

        let mut data_line = String::new();
        let due_date_str = self
            .task
            .due_date
            .to_string_format(self.not_american_format);

        if !due_date_str.is_empty() {
            data_line.push_str(&format!("📅 {due_date_str} "));
        }
        if self.task.priority > 0 {
            data_line.push_str(&format!("❗{} ", self.task.priority));
        }
        if !data_line.is_empty() {
            lines.push(Line::from(Span::styled(data_line, Style::default())));
        }
        let mut tag_line = String::new();
        if self.task.tags.is_some() {
            tag_line.push_str(
                &self
                    .task
                    .tags
                    .unwrap()
                    .iter()
                    .map(|t| format!("#{t}"))
                    .collect::<Vec<String>>()
                    .join(" "),
            );
        }
        if !tag_line.is_empty() {
            lines.push(Line::from(Span::styled(tag_line, Color::DarkGray)));
        }
        let description = self.task.description.unwrap_or_default();
        if !description.is_empty() {
            description
                .split('\n')
                .map(|l| Line::from(Span::styled(l, Color::Gray)))
                .for_each(|l| {
                    lines.push(l);
                });
        }

        if lines.is_empty() {
            let p = Paragraph::new(title);
            p.render(area, buf);
        } else {
            let text = Text::from(lines);

            let p = Paragraph::new(text).block(surrounding_block);
            p.render(area, buf);
        }
    }
}
