use color_eyre::Result;
use ratatui::{
    layout::{Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style, Stylize},
    widgets::calendar::{CalendarEventStore, Monthly},
    Frame,
};
use time::{Date, Month};

#[derive(Default, Clone, Copy)]
pub struct StyledCalendar;

impl StyledCalendar {
    pub fn render_year(frame: &mut Frame, area: Rect, date: Date) -> Result<()> {
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
            StyledCalendar::render_month(frame, area, month, &events);
        }
        Ok(())
    }

    fn render_month(frame: &mut Frame, area: Rect, date: Date, events: &CalendarEventStore) {
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
