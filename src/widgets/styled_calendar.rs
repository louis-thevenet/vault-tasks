use ratatui::{
    layout::{Constraint, Layout, Margin, Rect},
    style::{Style, Stylize},
    widgets::calendar::{CalendarEventStore, Monthly},
    Frame,
};
use time::{Date, Month};

#[derive(Default, Clone, Copy)]
pub struct StyledCalendar;

impl StyledCalendar {
    // pub fn render_year(frame: &mut Frame, area: Rect, date: Date, events: &CalendarEventStore) {
    //     let area = area.inner(Margin {
    //         vertical: 1,
    //         horizontal: 1,
    //     });
    //     let rows = Layout::vertical([Constraint::Ratio(1, 3); 3]).split(area);
    //     let areas = rows.iter().flat_map(|row| {
    //         Layout::horizontal([Constraint::Ratio(1, 4); 4])
    //             .split(*row)
    //             .to_vec()
    //     });
    //     for (i, area) in areas.enumerate() {
    //         let month = date
    //             .replace_day(1)
    //             .unwrap()
    //             .replace_month(Month::try_from(i as u8 + 1).unwrap())
    //             .unwrap();
    //         StyledCalendar::render_month(frame, area, month, events);
    //     }
    // }

    pub fn render_quarter(frame: &mut Frame, area: Rect, date: Date, events: &CalendarEventStore) {
        let area = area.inner(Margin {
            vertical: 1,
            horizontal: 1,
        });
        let [pred, cur, next] = Layout::vertical([Constraint::Length(2 + 5 + 1); 3]).areas(area);

        let mut prev_date = date;
        if date.month() == Month::January {
            prev_date = prev_date.replace_year(date.year() - 1).unwrap();
        }
        StyledCalendar::render_month(
            frame,
            pred,
            prev_date
                .replace_day(1)
                .unwrap()
                .replace_month(date.month().previous())
                .unwrap(),
            events,
        );
        StyledCalendar::render_month(frame, cur, date.replace_day(1).unwrap(), events);
        let mut next_date = date;
        if date.month() == Month::December {
            next_date = next_date.replace_year(date.year() + 1).unwrap();
        }
        StyledCalendar::render_month(
            frame,
            next,
            next_date
                .replace_day(1)
                .unwrap()
                .replace_month(date.month().next())
                .unwrap(),
            events,
        );
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
