use std::rc::Rc;

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};
use ratskin::RatSkin;
use tracing::error;

use crate::core::{PrettySymbolsConfig, task::Task, vault_data::VaultData};

const HEADER_INDENT_RATIO: u16 = 3;

#[derive(Clone)]
pub struct TaskListItem {
    item: VaultData,
    pub height: u16,
    symbols: PrettySymbolsConfig,
    not_american_format: bool,
    completion_bar_length: usize,
    show_relative_due_dates: bool,
    max_width: u16,
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
        symbols: PrettySymbolsConfig,
        max_width: u16,
        display_filename: bool,
        show_relative_due_dates: bool,

        completion_bar_length: usize,
    ) -> Self {
        let height = Self::compute_height(&item, max_width);
        Self {
            item,
            height,
            not_american_format,
            max_width,
            display_filename,
            symbols,
            header_style: Style::default(),
            show_relative_due_dates,
            completion_bar_length,
        }
    }
    #[allow(clippy::too_many_lines)]
    fn task_to_paragraph(&self, area: Rect, task: &Task) -> (Rc<[Rect]>, Paragraph<'_>) {
        let mut lines = vec![];
        let mut data_line = vec![];

        let rat_skin = RatSkin::default();

        let state = task.state.display(self.symbols.clone());
        let title = state.clone() + " " + &task.name;
        let title_parsed = rat_skin.parse(RatSkin::parse_text(&title), self.max_width);
        let binding = Line::raw(state);
        let title = match title_parsed.first() {
            Some(t) => {
                lines.append(&mut title_parsed[1..].to_vec());
                t
            }
            None => &binding,
        };

        let surrounding_block =
            Block::default()
                .borders(Borders::ALL)
                .title_bottom(if self.display_filename {
                    Line::from(task.filename.clone()).right_aligned()
                } else {
                    Line::from("")
                });

        if task.is_today {
            data_line.push(Span::raw(format!("{} ", self.symbols.today_tag)));
        }

        if let Some(due_date) = &task.due_date {
            data_line.push(Span::from(format!(
                "{} ",
                due_date.to_display_format(&self.symbols.due_date, self.not_american_format)
            )));
            if self.show_relative_due_dates {
                if let Some(due_date_relative) = due_date.get_relative_str() {
                    data_line.push(Span::styled(
                        format!("({due_date_relative}) "),
                        Style::new().dim(),
                    ));
                }
            }
        }
        if let Some(bar) = task.get_completion_bar(
            self.completion_bar_length,
            &(
                self.symbols.progress_bar_false.clone(),
                self.symbols.progress_bar_true.clone(),
            ),
        ) {
            data_line.push(Span::raw(bar));
        }
        if task.priority > 0 {
            data_line.push(Span::raw(format!(
                "{}{} ",
                self.symbols.priority, task.priority
            )));
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
            let text = rat_skin.parse(RatSkin::parse_text(&description), self.max_width);
            lines = [lines, text].concat();
        }
        let mut constraints = vec![Constraint::Length((lines.len()).try_into().unwrap())];

        for st in &task.subtasks {
            constraints.push(Constraint::Length(Self::compute_height(
                &VaultData::Task(st.clone()),
                self.max_width - 2, // -2 for borders
            )));
        }

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(surrounding_block.inner(area));

        (
            layout,
            if lines.is_empty() && task.subtasks.is_empty() {
                Paragraph::new(title.clone()).block(surrounding_block)
            } else {
                Paragraph::new(Text::from(lines)).block(surrounding_block.title_top(title.clone()))
            },
        )
    }
    fn compute_height(item: &VaultData, max_width: u16) -> u16 {
        let rat_skin = RatSkin::default();
        match &item {
            VaultData::Directory(_, _) => 1,
            VaultData::Header(_, _, children) => {
                children
                    .iter()
                    .map(|c| Self::compute_height(c, max_width * (100 - HEADER_INDENT_RATIO) / 100))
                    .sum::<u16>()
                    + 1 // name in block (border only on top)
            }
            VaultData::Task(task) => {
                let mut count: u16 = 2; // block
                if 2 + task.name.len() >= max_width as usize {
                    count += (2 + task.name.len() as u16) / max_width;
                }
                if let Some(d) = &task.description {
                    let parsed_desc = rat_skin.parse(RatSkin::parse_text(d), max_width);
                    count += u16::try_from(parsed_desc.len()).unwrap_or_else(|e| {
                        error!("Could not convert description length to u16 :{e}");
                        0
                    });
                }
                if task.due_date.is_some()
                    || task.priority > 0
                    || task.is_today
                    || task.completion.is_some()
                {
                    count += 1;
                }
                if task.tags.is_some() {
                    count += 1;
                }
                for sb in &task.subtasks {
                    count += Self::compute_height(&VaultData::Task(sb.clone()), max_width - 2);
                }
                count.max(3) // If count == 2 then task name will go directly inside a block
                // Else task name will be the block's title and content will go inside
            }
        }
    }
}
impl Widget for TaskListItem {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let rat_skin = RatSkin::default();
        match &self.item {
            VaultData::Directory(name, _) => error!("TaskList widget received a directory: {name}"),
            VaultData::Header(_level, name, children) => {
                let surrounding_block = Block::default().borders(Borders::TOP).title(
                    rat_skin
                        .parse(RatSkin::parse_text(name), area.width)
                        .first()
                        .unwrap()
                        .clone()
                        .style(self.header_style),
                );

                let indent = Layout::new(
                    Direction::Horizontal,
                    vec![
                        Constraint::Percentage(HEADER_INDENT_RATIO),
                        Constraint::Percentage(100 - HEADER_INDENT_RATIO),
                    ],
                )
                .split(area);

                let mut constraints = vec![];
                for child in children {
                    constraints.push(Constraint::Length(Self::compute_height(
                        child,
                        self.max_width - indent[0].width,
                    )));
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
                        self.symbols.clone(),
                        self.max_width - indent[0].width,
                        self.display_filename,
                        self.show_relative_due_dates,
                        self.completion_bar_length,
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
                        self.symbols.clone(),
                        self.max_width - 2, // surrounding block
                        false,
                        self.show_relative_due_dates,
                        self.completion_bar_length,
                    )
                    .header_style(self.header_style);

                    sb_widget.render(layout[i + 1], buf);
                }
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        core::{
            date::Date,
            task::{State, Task},
            vault_data::VaultData,
        },
        widgets::task_list_item::TaskListItem,
    };
    use chrono::NaiveDate;
    use insta::assert_snapshot;
    use ratatui::{Terminal, backend::TestBackend};

    use crate::config::Config;

    #[test]
    fn test_task_list_item() {
        let test_task = VaultData::Task(Task {
            name: "task with a very long title that should wrap to the next line".to_string(),
            state: State::Done,
            tags: Some(vec![String::from("tag"), String::from("tag2")]),
            priority: 5,
            completion: Some(60),
            due_date: Some(Date::DayTime(
                NaiveDate::from_ymd_opt(2016, 7, 8)
                    .unwrap()
                    .and_hms_opt(9, 10, 11)
                    .unwrap(),
            )),
            subtasks: vec![
                Task {
                    name: "subtask with another long title that should wrap around".to_string(),
                    description: Some("test\ndesc".to_string()),
                    ..Default::default()
                },
                Task {
                    name: "subtask test".to_string(),
                    tags: Some(vec![String::from("tag"), String::from("tag2")]),
                    ..Default::default()
                },
                Task {
                    name: "subtask test with a long title 123456789 1 2 3".to_string(),
                    priority: 5,
                    due_date: Some(Date::DayTime(
                        NaiveDate::from_ymd_opt(2016, 7, 8)
                            .unwrap()
                            .and_hms_opt(9, 10, 11)
                            .unwrap(),
                    )),
                    description: Some("test\ndesc".to_string()),
                    ..Default::default()
                },
            ],
            ..Default::default()
        });
        let mut config = Config::default();

        // We don't want tests to be time dependent
        config.tasks_config.show_relative_due_dates = false;

        let max_width = 50;
        let task_list_item = TaskListItem::new(
            test_task,
            false,
            config.tasks_config.pretty_symbols,
            max_width,
            false,
            false,
            config.tasks_config.completion_bar_length,
        );
        let mut terminal = Terminal::new(TestBackend::new(max_width, 40)).unwrap();
        terminal
            .draw(|frame| {
                frame.render_widget(task_list_item, frame.area());
            })
            .unwrap();
        assert_snapshot!(terminal.backend());
    }
}
