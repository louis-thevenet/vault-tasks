use ::time::{Date, OffsetDateTime};
use chrono::{Datelike, Duration, NaiveDate, NaiveTime};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{calendar::CalendarEventStore, StatefulWidget},
    Frame,
};
use time::{util::days_in_year, Weekday};
use tui_scrollview::ScrollViewState;

use crate::{
    action::Action,
    app::Mode,
    config::Config,
    core::{
        filter::{filter_to_vec, Filter},
        sorter::SortingMode,
        task::{DueDate, Task},
        vault_data::VaultData,
        TaskManager,
    },
    widgets::{help_menu::HelpMenu, styled_calendar::StyledCalendar, task_list::TaskList},
};

use super::Component;

/// Struct that helps with drawing the component
struct TimelineTabArea {
    calendar: Rect,
    footer: Rect,
    timeline: Rect,
}

pub struct TimelineTab<'a> {
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
impl<'a> Default for TimelineTab<'a> {
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
impl<'a> TimelineTab<'a> {
    pub fn new() -> Self {
        Self::default()
    }
    fn split_frame(area: Rect) -> TimelineTabArea {
        let [_header, content, footer, _tab_footera] = Layout::vertical([
            Constraint::Length(1), // tabs
            Constraint::Min(0),    // content
            Constraint::Length(1), //footer
            Constraint::Length(1), // home footer
        ])
        .areas(area);

        let [calendar, timeline] = Layout::horizontal([
            Constraint::Length(4 * (7 * 3 + 1)), // calendar
            Constraint::Min(0),                  // timeline
        ])
        .areas(content);

        TimelineTabArea {
            calendar,
            footer,
            timeline,
        }
    }
    fn render_footer(area: Rect, frame: &mut Frame) {
        ratatui::widgets::Widget::render(
            Line::raw("Place holder").centered(),
            area,
            frame.buffer_mut(),
        );
    }
    fn update_tasks(&mut self) {
        self.tasks = filter_to_vec(&self.task_mgr.tasks, &Filter::default());
        self.tasks.sort_by(SortingMode::cmp_due_date);

        self.entries_list = TaskList::new(
            &self.config,
            &self
                .tasks
                .clone()
                .iter()
                .map(|t| VaultData::Task(t.clone()))
                .collect::<Vec<VaultData>>(),
            true,
        );
    }
    fn updated_date(&mut self) {
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
        self.task_list_widget_state.scroll_to_top();
        (0..self.entries_list.height_of(index_closest_task)).for_each(|_| {
            self.task_list_widget_state.scroll_down();
        });
        self.tasks_to_events(&self.tasks[index_closest_task].clone());
    }
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
    fn tasks_to_events(&mut self, previewed_task: &Task) {
        const SELECTED: Style = Style::new()
            .fg(Color::White)
            .bg(Color::Red)
            .add_modifier(Modifier::BOLD);
        const PREVIEWED: Style = Style::new()
            .fg(Color::White)
            .bg(Color::Green)
            .add_modifier(Modifier::BOLD);
        const TASK: Style = Style::new()
            .fg(Color::Red)
            .add_modifier(Modifier::UNDERLINED);

        self.events = CalendarEventStore::today(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::Blue),
        );

        for task in self.tasks.clone() {
            let theme = if task == *previewed_task {
                PREVIEWED
            } else {
                TASK
            };
            match task.clone().due_date {
                DueDate::NoDate => (),
                DueDate::Day(naive_date) => {
                    let date = Self::naive_date_to_date(naive_date);
                    if !self.events.0.contains_key(&date) {
                        self.events.add(date, theme);
                    }
                }
                DueDate::DayTime(naive_datetime) => {
                    let date = Self::naive_date_to_date(naive_datetime.date());
                    if !self.events.0.contains_key(&date) {
                        self.events.add(date, theme);
                    }
                }
            }
        }

        // selected date
        self.events.add(self.selected_date, SELECTED);
    }
}
impl<'a> Component for TimelineTab<'a> {
    fn register_config_handler(&mut self, config: Config) -> color_eyre::eyre::Result<()> {
        self.task_mgr = TaskManager::load_from_config(&config.tasks_config)?;
        self.config = config;

        self.update_tasks();
        self.updated_date();
        self.help_menu_wigdet = HelpMenu::new(Mode::Timeline, &self.config);
        Ok(())
    }

    fn update(
        &mut self,
        _tui: Option<&mut crate::tui::Tui>,
        action: crate::action::Action,
    ) -> color_eyre::eyre::Result<Option<crate::action::Action>> {
        if !self.is_focused {
            match action {
                Action::Focus(Mode::Timeline) => self.is_focused = true,
                Action::Focus(mode) if !(mode == Mode::Timeline) => self.is_focused = false,
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
                Action::Focus(mode) if mode != Mode::Timeline => self.is_focused = false,
                Action::Focus(Mode::Timeline) => self.is_focused = true,
                Action::Help => self.show_help = !self.show_help,
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
        StyledCalendar::render_year(frame, areas.calendar, self.selected_date, &self.events);
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
