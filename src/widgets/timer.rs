use std::default;

use chrono::{NaiveDateTime, NaiveTime};
use ratatui::widgets::{Paragraph, StatefulWidget, Widget};
pub struct TimerWidget;

#[derive(Default)]
pub enum TimerState {
    ClockDown {
        stop_at: NaiveTime,
    },
    ClockUp {
        started_at: NaiveTime,
    },
    #[default]
    NotInitialized,
}

impl StatefulWidget for TimerWidget {
    type State = TimerState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let now = chrono::Local::now().time();
        match state {
            TimerState::ClockUp { started_at } => {
                let current = now - *started_at;
                Paragraph::new(format!("{current:?}"))
                    .centered()
                    .render(area, buf);
            }
            TimerState::ClockDown { stop_at } => {
                let remaining = *stop_at - now;
                Paragraph::new(format!("{remaining:?}"))
                    .centered()
                    .render(area, buf);
            }
            TimerState::NotInitialized => Paragraph::new(format!("Not initialized"))
                .centered()
                .render(area, buf),
        }
    }
}
