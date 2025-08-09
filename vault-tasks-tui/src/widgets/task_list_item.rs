use std::rc::Rc;

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};
use ratskin::RatSkin;
use tracing::error;
use vault_tasks_core::{config::TasksConfig, task::Task, tracker::Tracker, vault_data::VaultData};

const HEADER_INDENT_RATIO: u16 = 3;

#[derive(Clone)]
pub struct TaskListItem {
    item: VaultData,
    pub height: u16,
    // TODO: here we use stuff that should be in TUI and not core
    config: TasksConfig,
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
        config: TasksConfig,
        max_width: u16,
        display_filename: bool,
    ) -> Self {
        let height = Self::compute_height(&item, max_width);
        Self {
            item,
            height,
            config,
            max_width,
            display_filename,
            header_style: Style::default(),
        }
    }
    #[allow(clippy::too_many_lines)]
    fn task_to_paragraph(&self, area: Rect, task: &Task) -> (Rc<[Rect]>, Paragraph<'_>) {
        let mut lines = vec![];
        let mut data_line = vec![];

        let rat_skin = RatSkin::default();

        let state = task.state.display(self.config.pretty_symbols.clone());
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
            data_line.push(Span::raw(format!(
                "{} ",
                self.config.pretty_symbols.today_tag
            )));
        }

        if let Some(due_date) = &task.due_date {
            data_line.push(Span::from(format!(
                "{} ",
                due_date.to_display_format(
                    &self.config.pretty_symbols.due_date,
                    !self.config.use_american_format,
                )
            )));
            if self.config.show_relative_due_dates {
                let due_date_relative = due_date.get_relative_str();
                data_line.push(Span::styled(
                    format!("({due_date_relative}) "),
                    Style::new().dim(),
                ));
            }
        }
        if let Some(bar) = task.get_completion_bar(
            self.config.completion_bar_length,
            &(
                self.config.pretty_symbols.progress_bar_false.clone(),
                self.config.pretty_symbols.progress_bar_true.clone(),
            ),
        ) {
            data_line.push(Span::raw(bar));
        }
        if task.priority > 0 {
            data_line.push(Span::raw(format!(
                "{}{} ",
                self.config.pretty_symbols.priority, task.priority
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
    fn tracker_to_table(&self, tracker: &Tracker) -> Table<'_> {
        let header = [
            vec!["Dates".to_owned()],
            tracker.categories.iter().map(|c| c.name.clone()).collect(),
        ]
        .concat()
        .into_iter()
        .map(Cell::from)
        .collect::<Row>()
        .style(self.header_style)
        .height(1);

        let mut date = tracker.start_date.clone();
        let rat_skin = RatSkin::default();
        let mut rows = (0..tracker.length)
            .map(|n| {
                let res = Row::new(
                    [
                        vec![Cell::from(
                            Span::raw(
                                date.to_string_format(!self.config.use_american_format)
                                    .to_string(),
                            ) + if self.config.show_relative_due_dates {
                                Span::raw(format!(" ({})", date.get_relative_str())).dim()
                            } else {
                                Span::raw("")
                            },
                        )],
                        tracker
                            .categories
                            .iter()
                            .map(|c| {
                                Cell::from(Text::from(
                                    rat_skin.parse(
                                        RatSkin::parse_text(
                                            &c.entries
                                                .get(n)
                                                .unwrap()
                                                .pretty_fmt(&self.config.pretty_symbols),
                                        ),
                                        self.max_width,
                                    ),
                                ))
                            })
                            .collect(),
                    ]
                    .concat(),
                );
                date = tracker.frequency.next_date(&date);
                res
            })
            .collect::<Vec<Row>>();
        if self.config.invert_tracker_entries {
            rows.reverse();
        }
        let mut date = tracker.start_date.clone();
        let widths = [
            vec![
                (0..tracker.length)
                    .map(|_n| {
                        let res = if self.config.show_relative_due_dates {
                            format!(
                                "{} ({})",
                                date.to_string_format(!self.config.use_american_format,),
                                date.get_relative_str()
                            )
                        } else {
                            date.to_string_format(!self.config.use_american_format)
                                .to_string()
                        }
                        .len() as u16;
                        date = tracker.frequency.next_date(&date);
                        res
                    })
                    .max()
                    .unwrap_or_default(),
            ],
            tracker
                .categories
                .iter()
                .map(|cat| {
                    (cat.entries
                        .iter()
                        .map(|ent| ent.to_string().len())
                        .max()
                        .unwrap_or_default())
                    .max(cat.name.len()) as u16
                })
                .collect::<Vec<u16>>(),
        ]
        .concat();
        Table::new(rows, widths)
            .header(header)
            .column_spacing(2)
            .block(Block::bordered().title(tracker.name.clone()))
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
            VaultData::Tracker(tracker) => {
                2 // block
                    + 1 // header
                    + tracker
                    .categories
                    .first()
                    .map_or(0, |c| c.entries.len() as u16) // number of entries
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
                        self.config.clone(),
                        self.max_width - indent[0].width,
                        self.display_filename,
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
                        self.config.clone(),
                        self.max_width - 2, // surrounding block
                        false,
                    )
                    .header_style(self.header_style);

                    sb_widget.render(layout[i + 1], buf);
                }
            }
            VaultData::Tracker(tracker) => {
                Widget::render(self.tracker_to_table(tracker), area, buf);
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
        vault_data::VaultData,
    };

    use crate::{config::Config, widgets::task_list_item::TaskListItem};

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
        let task_list_item =
            TaskListItem::new(test_task, config.tasks_config.clone(), max_width, false);
        let mut terminal = Terminal::new(TestBackend::new(max_width, 40)).unwrap();
        terminal
            .draw(|frame| {
                frame.render_widget(task_list_item, frame.area());
            })
            .unwrap();
        assert_snapshot!(terminal.backend());
    }
}
