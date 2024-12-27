use color_eyre::Result;

use ::time::{Date, OffsetDateTime};
use ratatui::{
    layout::{Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::Line,
    widgets::{
        calendar::{CalendarEventStore, Monthly},
        ListState, StatefulWidget, Widget,
    },
    Frame,
};
use time::{Duration, Month};

use crate::{action::Action, app::Mode, config::Config, widgets::help_menu::HelpMenu};

use super::Component;

/// Struct that helps with drawing the component
struct TimelineTabArea {
    calendar: Rect,
    footer: Rect,
    timeline: Rect,
}

pub struct TimelineTab<'a> {
    config: Config,
    is_focused: bool,
    calendar: StyledCalendar,
    calendar_mode: ListState,
    selected_date: Date,
    // Whether the help panel is open or not
    show_help: bool,
    help_menu_wigdet: HelpMenu<'a>,
}
impl<'a> Default for TimelineTab<'a> {
    fn default() -> Self {
        // let now = chrono::Local::now();
        // let selected_date =OffsetDateTime:: Date::from_calendar_date(
        //     now.year(),
        //     Month::try_from(now.month() as u8).unwrap(),
        //     now.day() as u8,
        // )
        // .unwrap();
        Self {
            selected_date: OffsetDateTime::now_local().unwrap().date(),
            calendar: StyledCalendar,
            config: Config::default(),
            is_focused: false,
            show_help: false,
            help_menu_wigdet: HelpMenu::default(),
            calendar_mode: ListState::default(),
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
        Line::raw("Place holder")
            .centered()
            .render(area, frame.buffer_mut());
    }
}
impl<'a> Component for TimelineTab<'a> {
    fn register_config_handler(&mut self, config: Config) -> color_eyre::eyre::Result<()> {
        self.config = config;
        self.calendar_mode.select(Some(0));
        self.calendar = StyledCalendar::default();
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
                Action::Left => self.selected_date -= Duration::days(1),
                Action::Down => self.selected_date += Duration::weeks(1),
                Action::Up => self.selected_date -= Duration::weeks(1),
                Action::Right => self.selected_date += Duration::days(1),
                Action::NextCalendarMode => self.calendar_mode.select_next(),
                Action::PreviousCalendarMode => self.calendar_mode.select_previous(),
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
        self.calendar
            .render_year(frame, areas.calendar, self.selected_date)
            .unwrap();

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

#[derive(Default, Clone, Copy)]
struct StyledCalendar;

impl StyledCalendar {
    fn render_year(self, frame: &mut Frame, area: Rect, date: Date) -> Result<()> {
        let events = events(date)?;

        let area = area.inner(Margin {
            vertical: 1,
            horizontal: 1,
        });
        let rows = Layout::vertical([Constraint::Ratio(1, 3); 3]).split(area);
        let areas = rows.iter().flat_map(|row| {
            Layout::horizontal([Constraint::Ratio(1, 4); 4])
                .split(*row)
                .to_vec()
        });
        for (i, area) in areas.enumerate() {
            let month = date
                .replace_day(1)
                .unwrap()
                .replace_month(Month::try_from(i as u8 + 1).unwrap())
                .unwrap();
            self.render_month(frame, area, month, &events);
        }
        Ok(())
    }

    fn render_month(self, frame: &mut Frame, area: Rect, date: Date, events: &CalendarEventStore) {
        let calendar = Monthly::new(date, events)
            .default_style(Style::new().bold())
            .show_month_header(Style::default())
            .show_surrounding(Style::new().dim())
            .show_weekdays_header(Style::new().bold().green());
        frame.render_widget(calendar, area);
    }
}

/// Makes a Selffor the current year.
fn events(selected_date: Date) -> Result<CalendarEventStore> {
    const SELECTED: Style = Style::new()
        .fg(Color::White)
        .bg(Color::Red)
        .add_modifier(Modifier::BOLD);
    const HOLIDAY: Style = Style::new()
        .fg(Color::Red)
        .add_modifier(Modifier::UNDERLINED);
    const SEASON: Style = Style::new()
        .fg(Color::Green)
        .bg(Color::Black)
        .add_modifier(Modifier::UNDERLINED);

    let mut list = CalendarEventStore::today(
        Style::default()
            .add_modifier(Modifier::BOLD)
            .bg(Color::Blue),
    );
    let y = selected_date.year();

    // new year's
    list.add(Date::from_calendar_date(y, Month::January, 1)?, HOLIDAY);
    // next new_year's for December "show surrounding"
    list.add(Date::from_calendar_date(y + 1, Month::January, 1)?, HOLIDAY);
    // groundhog day
    list.add(Date::from_calendar_date(y, Month::February, 2)?, HOLIDAY);
    // april fool's
    list.add(Date::from_calendar_date(y, Month::April, 1)?, HOLIDAY);
    // earth day
    list.add(Date::from_calendar_date(y, Month::April, 22)?, HOLIDAY);
    // star wars day
    list.add(Date::from_calendar_date(y, Month::May, 4)?, HOLIDAY);
    // festivus
    list.add(Date::from_calendar_date(y, Month::December, 23)?, HOLIDAY);
    // new year's eve
    list.add(Date::from_calendar_date(y, Month::December, 31)?, HOLIDAY);

    // seasons
    // spring equinox
    list.add(Date::from_calendar_date(y, Month::March, 22)?, SEASON);
    // summer solstice
    list.add(Date::from_calendar_date(y, Month::June, 21)?, SEASON);
    // fall equinox
    list.add(Date::from_calendar_date(y, Month::September, 22)?, SEASON);
    // winter solstice
    list.add(Date::from_calendar_date(y, Month::December, 21)?, SEASON);

    // selected date
    list.add(selected_date, SELECTED);

    Ok(list)
}
