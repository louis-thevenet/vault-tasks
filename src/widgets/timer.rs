use chrono::{NaiveTime, TimeDelta};
use clap::builder::Str;
use color_eyre::eyre::Result;
use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    widgets::{Block, Gauge, StatefulWidget, Widget},
};
use tracing::debug;
pub struct TimerWidget;

#[derive(Default)]
pub enum TimerState {
    ClockDown {
        started_at: NaiveTime,
        stop_at: NaiveTime,
    },
    ClockUp {
        started_at: NaiveTime,
    },
    #[default]
    NotInitialized,
}
impl TimerState {
    /// Returns true if the current time finished
    pub fn tick(&self) -> bool {
        match self {
            TimerState::ClockDown {
                started_at: _,
                stop_at,
            } => chrono::Local::now().time() > *stop_at,
            TimerState::ClockUp { started_at: _ } | TimerState::NotInitialized => false,
        }
    }
}
impl TimerWidget {
    fn format_time_delta(td: TimeDelta) -> String {
        let mut res = String::new();
        if td.num_hours() > 0 {
            res.push_str(&format!("{:02}", td.num_hours()));
        }
        res.push_str(&format!("{:02}:{:02}", td.num_minutes(), td.num_seconds()));
        res
    }
}

impl StatefulWidget for TimerWidget {
    type State = TimerState;
    #[allow(clippy::cast_precision_loss)]
    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let now = chrono::Local::now().time();
        let text = match state {
            TimerState::ClockUp { started_at } => {
                let current = now - *started_at;
                Self::format_time_delta(current)
            }
            TimerState::ClockDown {
                stop_at,
                started_at: _,
            } => {
                let remaining = *stop_at - now;
                Self::format_time_delta(remaining)
            }
            TimerState::NotInitialized => "Not initialized".to_string(),
        };

        // Create a centered box
        let [area] = Layout::horizontal([Constraint::Length(2 + 4 * (text.len()) as u16)]) // +2 for the block
            .flex(Flex::Center)
            .areas(area);
        let [area] = Layout::vertical([Constraint::Length(2 + 3)])
            .flex(Flex::Center)
            .areas(area);

        let ratio = match state {
            TimerState::ClockDown {
                stop_at,
                started_at,
            } => {
                let num = (now - *started_at).abs().to_std().unwrap().as_nanos() as f64;
                let den = (*stop_at - *started_at).abs().to_std().unwrap().as_nanos() as f64;
                1.0_f64.min(num / den)
            }
            TimerState::ClockUp { started_at: _ } | TimerState::NotInitialized => 1.0,
        };
        Gauge::default()
            .block(Block::bordered())
            .ratio(ratio)
            .label(text)
            .render(area, buf);
    }
}
