use std::collections::hash_map::Entry;

use ::time::{Date, OffsetDateTime};
use chrono::{Datelike, Duration, NaiveDate, NaiveTime};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, ToSpan},
    widgets::{calendar::CalendarEventStore, StatefulWidget, Widget},
    Frame,
};
use time::{util::days_in_year, Weekday};
use tracing::error;
use tui_scrollview::ScrollViewState;

use crate::{
    action::Action,
    app::Mode,
    config::Config,
    core::{
        filter::{filter_to_vec, Filter},
        sorter::SortingMode,
        task::{DueDate, State, Task},
        vault_data::VaultData,
        TaskManager,
    },
    widgets::{help_menu::HelpMenu, styled_calendar::StyledCalendar, task_list::TaskList},
};

use super::Component;

/// Struct that helps with drawing the component
struct CalendarTabArea {
    date: Rect,
    calendar: Rect,
    legend: Rect,
    footer: Rect,
    timeline: Rect,
}

pub struct CalendarTab<'a> {
    // Utils
    config: Config,
    is_focused: bool,
    task_mgr: TaskManager,
    // Content
    tasks: Vec<Task>,
    entries_list: TaskList,
    events: CalendarEventStore,
    selected_date: Date,
    task_list_widget_state: ScrollViewState,
    // Whether the help panel is open or not
    show_help: bool,
    help_menu_wigdet: HelpMenu<'a>,
}
impl Default for CalendarTab<'_> {
    fn default() -> Self {
        Self {
            selected_date: OffsetDateTime::now_local().unwrap().date(),
            config: Config::default(),
            is_focused: false,
            show_help: false,
            help_menu_wigdet: HelpMenu::default(),
            tasks: vec![],
            task_mgr: TaskManager::default(),
            task_list_widget_state: ScrollViewState::new(),
            entries_list: TaskList::default(),
            events: CalendarEventStore::default(),
        }
    }
}
impl CalendarTab<'_> {
    const SELECTED: Style = Style::new()
        .fg(Color::White)
        .bg(Color::Red)
        .add_modifier(Modifier::BOLD);
    const PREVIEWED: Style = Style::new()
        .fg(Color::White)
        .bg(Color::Green)
        .add_modifier(Modifier::BOLD);
    const TASK_DONE: Style = Style::new()
        .fg(Color::Green)
        .add_modifier(Modifier::UNDERLINED);
    const TASK_TODO: Style = Style::new()
        .fg(Color::Red)
        .add_modifier(Modifier::UNDERLINED);
    pub fn new() -> Self {
        Self::default()
    }
    fn split_frame(area: Rect) -> CalendarTabArea {
        let [_header, content, footer, _tab_footera] = Layout::vertical([
            Constraint::Length(1), // tabs
            Constraint::Min(0),    // content
            Constraint::Length(1), //footer
            Constraint::Length(1), // home footer
        ])
        .areas(area);

        let [calendar, timeline] = Layout::horizontal([
            Constraint::Length(7 * 3 + 5 + 4), // calendar
            Constraint::Min(0),                // timeline
        ])
        .areas(content);
        let [calendar, legend] = Layout::vertical([
            Constraint::Length(7 * 3 + 5), // calendar
            Constraint::Min(0),            // legend
        ])
        .areas(calendar);

        let [date, timeline] = Layout::vertical([
            Constraint::Length(1), // date
            Constraint::Min(0),    // timeline
        ])
        .areas(timeline);

        CalendarTabArea {
            date,
            calendar,
            legend,
            footer,
            timeline,
        }
    }
    fn render_footer(area: Rect, frame: &mut Frame) {
        ratatui::widgets::Widget::render(
            Line::raw("Navigate: <hjkl|◄▼▲▶> | Month: Shift+<jk|▼▲> | Goto Today: <t>").centered(),
            area,
            frame.buffer_mut(),
        );
    }
    fn update_tasks(&mut self) {
        // Gather tasks to vector
        self.tasks = filter_to_vec(&self.task_mgr.tasks, &Filter::default());
        self.tasks.sort_by(SortingMode::cmp_due_date);
    }
    fn updated_date(&mut self) {
        // Find a task to preview
        let mut index_closest_task = 0;
        let mut best = Duration::max_value();
        for (i, task) in self.tasks.iter().enumerate() {
            let d = match task.due_date {
                DueDate::NoDate => Duration::max_value(),
                DueDate::Day(naive_date) => NaiveDate::from_ymd_opt(
                    self.selected_date.year(),
                    self.selected_date.month() as u32,
                    u32::from(self.selected_date.day()),
                )
                .unwrap()
                .signed_duration_since(naive_date)
                .abs(),
                DueDate::DayTime(naive_date_time) => NaiveDate::from_ymd_opt(
                    self.selected_date.year(),
                    self.selected_date.month() as u32,
                    u32::from(self.selected_date.day()),
                )
                .unwrap()
                .and_time(NaiveTime::default())
                .signed_duration_since(naive_date_time)
                .abs(),
            };
            if d < best {
                best = d;
                index_closest_task = i;
            }
        }

        // Build preview task list
        let tasks_to_preview = if self.tasks.get(index_closest_task).is_some() {
            &filter_to_vec(
                &self.task_mgr.tasks,
                &Filter::new(
                    Task {
                        due_date: self.tasks[index_closest_task].due_date.clone(),
                        ..Default::default()
                    },
                    None,
                ),
            )
            .iter()
            .map(|t| VaultData::Task(t.clone()))
            .collect::<Vec<VaultData>>()
        } else {
            &vec![]
        };
        self.entries_list = TaskList::new(&self.config, tasks_to_preview, 200, true);
        self.task_list_widget_state.scroll_to_top(); // reset view
        self.tasks_to_events(self.tasks.clone().get(index_closest_task));
    }
    #[allow(clippy::cast_possible_truncation)]
    fn naive_date_to_date(naive_date: NaiveDate) -> Date {
        Date::from_iso_week_date(
            naive_date.year(),
            naive_date.iso_week().week() as u8,
            match naive_date.weekday() {
                chrono::Weekday::Mon => Weekday::Monday,
                chrono::Weekday::Tue => Weekday::Tuesday,
                chrono::Weekday::Wed => Weekday::Wednesday,
                chrono::Weekday::Thu => Weekday::Thursday,
                chrono::Weekday::Fri => Weekday::Friday,
                chrono::Weekday::Sat => Weekday::Saturday,
                chrono::Weekday::Sun => Weekday::Sunday,
            },
        )
        .unwrap()
    }
    fn tasks_to_events(&mut self, previewed_task: Option<&Task>) {
        self.events = CalendarEventStore::today(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::Blue),
        );
        // Previewed date
        if let Some(t) = previewed_task {
            match t.due_date {
                DueDate::NoDate => (),
                DueDate::Day(naive_date) => self
                    .events
                    .add(Self::naive_date_to_date(naive_date), Self::PREVIEWED),

                DueDate::DayTime(naive_date_time) => self.events.add(
                    Self::naive_date_to_date(naive_date_time.date()),
                    Self::PREVIEWED,
                ),
            }
        }
        // selected date
        self.events.add(self.selected_date, Self::SELECTED);

        let mut current = None;
        for task in self.tasks.clone() {
            let next = match task.clone().due_date {
                DueDate::NoDate => None,
                DueDate::Day(naive_date) => Some(Self::naive_date_to_date(naive_date)),

                DueDate::DayTime(naive_datetime) => {
                    Some(Self::naive_date_to_date(naive_datetime.date()))
                }
            };
            let theme = match task.state {
                State::ToDo | State::Incomplete => Self::TASK_TODO,
                State::Done | State::Canceled => Self::TASK_DONE,
            };
            if let Some(date) = next {
                // Already marked as selected
                if date == self.selected_date
                    || self
                        .events
                        .0
                        .get(&date)
                        .is_some_and(|&t| t == Self::PREVIEWED)
                {
                    self.events.0.insert(
                        date,
                        self.events
                            .0
                            .get(&date)
                            .unwrap()
                            .add_modifier(Modifier::UNDERLINED),
                    );
                }

                // Are we on the same date as before ?
                if current.is_some_and(|d: Date| d == date) {
                    // update if needed
                    if let Entry::Occupied(mut e) = self.events.0.entry(date) {
                        if theme == Self::TASK_TODO {
                            e.insert(theme); // Todo has priority over Done
                        }
                    } else {
                        error!("No event on this date but tasks exist");
                    }
                }
                if self.events.0.contains_key(&date) {
                    error!("Calendar entry exists but no tasks were added yet");
                } else {
                    self.events.add(date, theme);
                    current = next;
                }
            }
        }
    }
    fn render_legend(areas: &CalendarTabArea, frame: &mut Frame<'_>) {
        let [todo, done, selected, previewed, today] =
            Layout::vertical([Constraint::Length(1); 5]).areas(areas.legend);
        ratatui::widgets::Widget::render(
            Span::raw("Todo")
                .style(Self::TASK_TODO)
                .into_left_aligned_line(),
            todo,
            frame.buffer_mut(),
        );
        ratatui::widgets::Widget::render(
            Span::raw("Done")
                .style(Self::TASK_DONE)
                .into_left_aligned_line(),
            done,
            frame.buffer_mut(),
        );
        ratatui::widgets::Widget::render(
            Span::raw("Selected")
                .style(Self::SELECTED)
                .into_left_aligned_line(),
            selected,
            frame.buffer_mut(),
        );
        ratatui::widgets::Widget::render(
            Span::raw("Previewed")
                .style(Self::PREVIEWED)
                .into_left_aligned_line(),
            previewed,
            frame.buffer_mut(),
        );
        ratatui::widgets::Widget::render(
            Span::raw("Today")
                .style(
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .bg(Color::Blue),
                )
                .into_left_aligned_line(),
            today,
            frame.buffer_mut(),
        );
    }
}
impl Component for CalendarTab<'_> {
    fn register_config_handler(&mut self, config: Config) -> color_eyre::eyre::Result<()> {
        self.task_mgr = TaskManager::load_from_config(&config.tasks_config)?;
        self.config = config;

        self.update_tasks();
        self.updated_date();
        self.help_menu_wigdet = HelpMenu::new(Mode::Calendar, &self.config);
        Ok(())
    }

    fn update(
        &mut self,
        _tui: Option<&mut crate::tui::Tui>,
        action: crate::action::Action,
    ) -> color_eyre::eyre::Result<Option<crate::action::Action>> {
        if !self.is_focused {
            match action {
                Action::ReloadVault => {
                    self.task_mgr.reload(&self.config.tasks_config)?;
                    self.update_tasks();
                    self.updated_date();
                }
                Action::Focus(Mode::Calendar) => self.is_focused = true,
                Action::Focus(mode) if !(mode == Mode::Calendar) => self.is_focused = false,
                _ => (),
            }
        } else if self.show_help {
            match action {
                Action::ViewUp | Action::Up => self.help_menu_wigdet.scroll_up(),
                Action::ViewDown | Action::Down => self.help_menu_wigdet.scroll_down(),
                Action::Help | Action::Escape | Action::Enter => {
                    self.show_help = !self.show_help;
                }
                _ => (),
            }
        } else {
            match action {
                Action::Focus(mode) if mode != Mode::Calendar => self.is_focused = false,
                Action::Focus(Mode::Calendar) => self.is_focused = true,
                Action::Help => self.show_help = !self.show_help,
                Action::GotoToday => {
                    self.selected_date = OffsetDateTime::now_local().unwrap().date();
                    self.updated_date();
                }
                Action::ReloadVault => {
                    self.task_mgr.reload(&self.config.tasks_config)?;
                    self.update_tasks();
                    self.updated_date();
                }
                Action::Left => {
                    self.selected_date -= time::Duration::days(1);

                    self.updated_date();
                }
                Action::Down => {
                    self.selected_date += time::Duration::weeks(1);
                    self.updated_date();
                }
                Action::Up => {
                    self.selected_date -= time::Duration::weeks(1);
                    self.updated_date();
                }
                Action::Right => {
                    self.selected_date += time::Duration::days(1);
                    self.updated_date();
                }
                Action::NextMonth => {
                    self.selected_date += time::Duration::days(i64::from(
                        self.selected_date.month().length(self.selected_date.year()),
                    ));
                    self.updated_date();
                }
                Action::PreviousMonth => {
                    self.selected_date -= time::Duration::days(i64::from(
                        self.selected_date.month().length(self.selected_date.year()),
                    ));
                    self.updated_date();
                }
                Action::NextYear => {
                    self.selected_date += time::Duration::days(i64::from(days_in_year(
                        self.selected_date.year() + 1,
                    )));
                    self.updated_date();
                }

                Action::PreviousYear => {
                    self.selected_date -= time::Duration::days(i64::from(days_in_year(
                        self.selected_date.year() + 1,
                    )));
                    self.updated_date();
                }
                Action::ViewUp => self.task_list_widget_state.scroll_up(),
                Action::ViewDown => self.task_list_widget_state.scroll_down(),
                Action::ViewPageUp => self.task_list_widget_state.scroll_page_up(),
                Action::ViewPageDown => self.task_list_widget_state.scroll_page_down(),
                Action::ViewRight => self.task_list_widget_state.scroll_right(),
                Action::ViewLeft => self.task_list_widget_state.scroll_left(),
                _ => (),
            }
        }
        Ok(None)
    }
    fn draw(
        &mut self,
        frame: &mut ratatui::Frame,
        area: ratatui::prelude::Rect,
    ) -> color_eyre::eyre::Result<()> {
        if !self.is_focused {
            return Ok(());
        }

        let areas = Self::split_frame(area);

        // Calendar
        StyledCalendar::render_quarter(frame, areas.calendar, self.selected_date, &self.events);

        // Legend
        Self::render_legend(&areas, frame);

        // Date
        self.selected_date
            .to_span()
            .bold()
            .render(areas.date, frame.buffer_mut());

        // Timeline
        self.entries_list.clone().render(
            areas.timeline,
            frame.buffer_mut(),
            &mut self.task_list_widget_state,
        );

        // Footer
        Self::render_footer(areas.footer, frame);
        // Help
        if self.show_help {
            self.help_menu_wigdet.clone().render(
                area,
                frame.buffer_mut(),
                &mut self.help_menu_wigdet.state,
            );
        }
        Ok(())
    }
}
