use std::time::Duration;

use better_panic::debug_install;
use chrono::{NaiveTime, TimeDelta};
use ratatui::widgets::{Block, Gauge, StatefulWidget, Widget};
pub struct TimerWidget;

#[derive(Default, Clone)]
pub enum TimerState {
    ClockDown {
        started_at: NaiveTime,
        stop_at: NaiveTime,
        paused_at: Option<NaiveTime>,
    },
    ClockUp {
        started_at: NaiveTime,
        paused_at: Option<NaiveTime>,
    },
    #[default]
    NotInitialized,
}
impl TimerState {
    pub fn new(new_duration: Option<Duration>) -> Self {
        let now = chrono::Local::now().time();
        match new_duration {
            Some(d) => Self::ClockDown {
                started_at: now,
                stop_at: now
                    .overflowing_add_signed(TimeDelta::from_std(d).unwrap_or_default())
                    .0,
                paused_at: None,
            },
            None => Self::ClockUp {
                started_at: now,
                paused_at: None,
            },
        }
    }
    pub fn pause(self) -> Self {
        let now = chrono::Local::now().time();
        match self {
            TimerState::ClockDown {
                started_at,
                stop_at,
                paused_at,
            } => {
                if let Some(paused_at) = paused_at {
                    let delta = now - paused_at;
                    Self::ClockDown {
                        started_at: started_at + delta,
                        stop_at: stop_at + delta,
                        paused_at: None,
                    }
                } else {
                    Self::ClockDown {
                        started_at,
                        stop_at,
                        paused_at: Some(now),
                    }
                }
            }
            TimerState::ClockUp {
                started_at,
                paused_at,
            } => {
                if let Some(paused_at) = paused_at {
                    let delta = now - paused_at;
                    Self::ClockUp {
                        paused_at: None,
                        started_at: started_at + delta,
                    }
                } else {
                    TimerState::ClockUp {
                        started_at,
                        paused_at: Some(now),
                    }
                }
            }
            TimerState::NotInitialized => TimerState::NotInitialized,
        }
    }
    pub fn get_time_spent(&self) -> Result<Duration, chrono::OutOfRangeError> {
        let now = chrono::Local::now().time();
        match self {
            TimerState::ClockUp {
                started_at,
                paused_at: _,
            }
            | TimerState::ClockDown {
                started_at,
                stop_at: _,
                paused_at: _,
            } => (now - *started_at).to_std(),
            TimerState::NotInitialized => Ok(Duration::ZERO),
        }
    }
    /// Returns true if the current time finished
    pub fn tick(&self) -> bool {
        match self {
            TimerState::ClockDown {
                started_at: _,
                stop_at,
                paused_at: paused,
            } => chrono::Local::now().time() > *stop_at && paused.is_none(),
            TimerState::ClockUp {
                started_at: _,
                paused_at: _,
            } => false,
            TimerState::NotInitialized => false,
        }
    }
}
impl TimerWidget {
    fn format_time_delta(td: TimeDelta) -> String {
        let seconds = td.num_seconds() % 60;
        let minutes = (td.num_seconds() / 60) % 60;
        let hours = (td.num_seconds() / 60) / 60;

        let mut res = String::new();
        if td.num_hours() > 0 {
            res.push_str(&format!("{hours:02}"));
        }
        res.push_str(&format!("{minutes:02}:{seconds:02}"));
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
            TimerState::NotInitialized => "Not initialized".to_string(),
            TimerState::ClockUp {
                started_at,
                paused_at: paused,
            } => {
                if paused.is_some() {
                    "Paused".to_string()
                } else {
                    let current = now - *started_at;
                    Self::format_time_delta(current)
                }
            }
            TimerState::ClockDown {
                stop_at,
                started_at: _,
                paused_at: paused,
            } => {
                if paused.is_some() {
                    "Paused".to_string()
                } else {
                    let remaining = *stop_at - now;
                    Self::format_time_delta(remaining)
                }
            }
        };

        // let [area] = Layout::vertical([Constraint::Length(2 + 3)]).areas(area);

        let ratio = match state {
            TimerState::ClockDown {
                stop_at,
                started_at,
                paused_at,
            } => {
                let delta = if let Some(paused_at) = paused_at {
                    now - *paused_at
                } else {
                    TimeDelta::zero()
                };
                let num = (now - *started_at - delta)
                    .abs()
                    .to_std()
                    .unwrap()
                    .as_nanos() as f64;
                let den = (*stop_at - *started_at).abs().to_std().unwrap().as_nanos() as f64;
                1.0_f64.min(num / den)
            }
            TimerState::ClockUp {
                started_at: _,
                paused_at: _,
            }
            | TimerState::NotInitialized => 1.0,
        };
        Gauge::default()
            .block(Block::bordered())
            .ratio(ratio)
            .label(text)
            .render(area, buf);
    }
}
