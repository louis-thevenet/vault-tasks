use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};
use tracing::error;

use crate::task_core::{
    task::{DueDate, PRIORITY_EMOJI},
    vault_data::VaultData,
};

pub struct TaskListItem {
    item: VaultData,
    pub height: usize,
    not_american_format: bool,
    display_filename: bool,
}

impl TaskListItem {
    pub fn new(item: VaultData, not_american_format: bool, display_filename: bool) -> Self {
        let height = Self::compute_height(&item);
        Self {
            item,
            height,
            not_american_format,
            display_filename,
        }
    }

    fn compute_height(item: &VaultData) -> usize {
        match &item {
            VaultData::Directory(_, _) => 1,
            VaultData::Header(_, _, children) => {
                children.iter().map(Self::compute_height).sum::<usize>() + 2 // name in block
            }
            VaultData::Task(task) => {
                let mut count = 0;
                if let Some(d) = &task.description {
                    count += d.split('\n').count();
                }
                if task.tags.is_some() {
                    count += 1;
                }
                if task.due_date != DueDate::NoDate || task.priority > 0 {
                    count += 1;
                }

                for sb in &task.subtasks {
                    count += Self::compute_height(&VaultData::Task(sb.clone()));
                }

                if count > 0 {
                    count + 2 // content + block
                } else {
                    3 // name + block
                }
            }
        }
    }
}
impl Widget for TaskListItem {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        match &self.item {
            VaultData::Directory(name, _) => error!("TaskList widget received a directory: {name}"),
            VaultData::Header(_level, name, children) => {
                let surrounding_block = Block::default()
                    .borders(Borders::TOP )
                    .title(Span::styled(name.to_string(), Style::new().bold().fg(Color::Rgb(255,153 ,0 ))))
                    // .style(
                    //     Style::new().fg(Color::Rgb(255, 153, 0)), // .bg(Color::Rgb(28, 28, 32)),
                    // )
                    ;

                let indent = Layout::new(
                    Direction::Horizontal,
                    vec![Constraint::Percentage(3), Constraint::Percentage(97)],
                )
                .split(area);

                let mut constraints = vec![];
                for child in children {
                    constraints.push(Constraint::Length(
                        Self::compute_height(child).try_into().unwrap(),
                    ));
                }
                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(constraints)
                    .split(surrounding_block.inner(indent[1]));

                surrounding_block.render(area, buf);

                for (i, child) in children.iter().enumerate() {
                    let sb_widget = Self::new(child.clone(), self.not_american_format, false);
                    sb_widget.render(layout[i], buf);
                }
            }
            VaultData::Task(task) => {
                let mut lines = vec![];
                let state = task.state.to_string();

                let title = Span::styled(format!("{state} {}", task.name), Style::default());
                let surrounding_block = Block::default().borders(Borders::ALL);

                let mut data_line = String::new();
                let due_date_str = task.due_date.to_display_format(self.not_american_format);

                if !due_date_str.is_empty() {
                    data_line.push_str(&format!("{due_date_str} "));
                }
                if task.priority > 0 {
                    data_line.push_str(&format!("{}{} ", PRIORITY_EMOJI, task.priority));
                }
                if !data_line.is_empty() {
                    lines.push(Line::from(Span::styled(data_line, Style::default())));
                }
                let mut tag_line = String::new();
                if task.tags.is_some() {
                    tag_line.push_str(
                        &task
                            .tags
                            .clone()
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
                if let Some(description) = task.description.clone() {
                    for l in description.lines() {
                        lines.push(Line::from(Span::styled(l.to_string(), Color::Gray)));
                    }
                }

                let mut constraints = vec![Constraint::Length((lines.len()).try_into().unwrap())];
                constraints.append(&mut vec![Constraint::Min(1); task.subtasks.len()]);
                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(constraints)
                    .split(surrounding_block.inner(area));

                if lines.is_empty() && task.subtasks.is_empty() {
                    Paragraph::new(title).block(surrounding_block)
                } else {
                    Paragraph::new(Text::from(lines)).block(
                        surrounding_block.title_top(title.clone()).title_bottom(
                            if self.display_filename {
                                Line::from(task.filename.clone()).right_aligned()
                            } else {
                                Line::from("")
                            },
                        ),
                    )
                }
                .render(area, buf);

                for (i, sb) in task.subtasks.iter().enumerate() {
                    let sb_widget =
                        Self::new(VaultData::Task(sb.clone()), self.not_american_format, false);
                    sb_widget.render(layout[i + 1], buf);
                }
            }
        };
    }
}
