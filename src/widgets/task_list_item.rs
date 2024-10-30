use std::rc::Rc;

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};
use tracing::error;

use crate::task_core::{
    task::{DueDate, Task, PRIORITY_EMOJI, TODAY_FLAG_EMOJI},
    vault_data::VaultData,
};

pub struct TaskListItem {
    item: VaultData,
    pub height: u16,
    not_american_format: bool,
    show_relative_due_dates: bool,
    display_filename: bool,
    header_style: Style,
}

impl TaskListItem {
    pub fn header_style(mut self, style: Style) -> Self {
        self.header_style = style;
        self
    }
    pub fn new(
        item: VaultData,
        not_american_format: bool,
        display_filename: bool,
        show_relative_due_dates: bool,
    ) -> Self {
        let height = Self::compute_height(&item);
        Self {
            item,
            height,
            not_american_format,
            display_filename,
            header_style: Style::default(),
            show_relative_due_dates,
        }
    }
    fn task_to_paragraph(&self, area: Rect, task: &Task) -> (Rc<[Rect]>, Paragraph<'_>) {
        let mut lines = vec![];
        let state = task.state.to_string();
        let title = Span::styled(format!("{state} {}", task.name), Style::default());
        let surrounding_block =
            Block::default()
                .borders(Borders::ALL)
                .title_bottom(if self.display_filename {
                    Line::from(task.filename.clone()).right_aligned()
                } else {
                    Line::from("")
                });

        let mut data_line = vec![];

        if task.is_today {
            data_line.push(Span::raw(format!("{TODAY_FLAG_EMOJI} ")));
        }

        let due_date_str = task.due_date.to_display_format(self.not_american_format);

        if !due_date_str.is_empty() {
            data_line.push(Span::from(format!("{due_date_str} ")));
            if self.show_relative_due_dates {
                if let Some(due_date_relative) = task.due_date.get_relative_str() {
                    data_line.push(Span::styled(
                        format!("({due_date_relative}) "),
                        Style::new().dim(),
                    ));
                }
            }
        }
        if task.priority > 0 {
            data_line.push(Span::raw(format!("{}{} ", PRIORITY_EMOJI, task.priority)));
        }
        if !data_line.is_empty() {
            lines.push(Line::from(data_line));
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

        for st in &task.subtasks {
            constraints.push(Constraint::Length(Self::compute_height(&VaultData::Task(
                st.clone(),
            ))));
        }

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(surrounding_block.inner(area));

        (
            layout,
            if lines.is_empty() && task.subtasks.is_empty() {
                Paragraph::new(title).block(surrounding_block)
            } else {
                Paragraph::new(Text::from(lines)).block(surrounding_block.title_top(title.clone()))
            },
        )
    }
    fn compute_height(item: &VaultData) -> u16 {
        match &item {
            VaultData::Directory(_, _) => 1,
            VaultData::Header(_, _, children) => {
                children.iter().map(Self::compute_height).sum::<u16>() + 1 // name in block (border only on top)
            }
            VaultData::Task(task) => {
                let mut count: u16 = 2; // block
                if let Some(d) = &task.description {
                    count += u16::try_from(d.split('\n').count()).unwrap_or_else(|e| {
                        error!("Could not convert description length to u16 :{e}");
                        0
                    });
                }
                if task.due_date != DueDate::NoDate || task.priority > 0 || task.is_today {
                    count += 1;
                }
                if task.tags.is_some() {
                    count += 1;
                }
                for sb in &task.subtasks {
                    count += Self::compute_height(&VaultData::Task(sb.clone()));
                }
                count.max(3) // If count == 2 then we add task name will be in the block
                             // Else name goes in block title
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
                    .borders(Borders::TOP)
                    .title(Span::styled(name.to_string(), self.header_style));

                let indent = Layout::new(
                    Direction::Horizontal,
                    vec![Constraint::Percentage(3), Constraint::Percentage(97)],
                )
                .split(area);

                let mut constraints = vec![];
                for child in children {
                    constraints.push(Constraint::Length(Self::compute_height(child)));
                }
                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(constraints)
                    .split(surrounding_block.inner(indent[1]));
                surrounding_block.render(area, buf);

                for (i, child) in children.iter().enumerate() {
                    let sb_widget = Self::new(
                        child.clone(),
                        self.not_american_format,
                        self.display_filename,
                        self.show_relative_due_dates,
                    )
                    .header_style(self.header_style);
                    sb_widget.render(layout[i], buf);
                }
            }
            VaultData::Task(task) => {
                let (layout, par) = self.task_to_paragraph(area, task);
                par.render(area, buf);

                for (i, sb) in task.subtasks.iter().enumerate() {
                    let sb_widget = Self::new(
                        VaultData::Task(sb.clone()),
                        self.not_american_format,
                        false,
                        self.show_relative_due_dates,
                    )
                    .header_style(self.header_style);

                    sb_widget.render(layout[i + 1], buf);
                }
            }
        };
    }
}
