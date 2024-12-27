use ::time::{Date, OffsetDateTime};
use color_eyre::Result;
use ratatui::{
    layout::{Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::Line,
    widgets::{
        calendar::{CalendarEventStore, Monthly},
        StatefulWidget, Widget,
    },
    Frame,
};
use time::{util::days_in_year, Duration, Month};

use crate::{
    action::Action,
    app::Mode,
    config::Config,
    widgets::{help_menu::HelpMenu, styled_calendar::StyledCalendar},
};

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
            config: Config::default(),
            is_focused: false,
            show_help: false,
            help_menu_wigdet: HelpMenu::default(),
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
                Action::NextMonth => {
                    self.selected_date += Duration::days(i64::from(
                        self.selected_date.month().length(self.selected_date.year()),
                    ));
                }
                Action::PreviousMonth => {
                    self.selected_date -= Duration::days(i64::from(
                        self.selected_date.month().length(self.selected_date.year()),
                    ));
                }
                Action::NextYear => {
                    self.selected_date +=
                        Duration::days(i64::from(days_in_year(self.selected_date.year() + 1)));
                }

                Action::PreviousYear => {
                    self.selected_date -=
                        Duration::days(i64::from(days_in_year(self.selected_date.year() + 1)));
                }
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
        StyledCalendar::render_year(frame, areas.calendar, self.selected_date).unwrap();

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
