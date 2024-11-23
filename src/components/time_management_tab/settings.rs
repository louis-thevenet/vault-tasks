use chrono::NaiveTime;
use color_eyre::Result;
use std::{fmt::Display, time::Duration};
use strum::{EnumIter, FromRepr};

use tracing::debug;

use crate::widgets::timer::TimerWidget;

#[derive(Default, Clone, Copy, FromRepr, EnumIter, strum_macros::Display, PartialEq, Eq, Hash)]
pub enum TimerTechniquesAvailable {
    #[default]
    #[strum(to_string = "Pomodoro")]
    Pomodoro,
    #[strum(to_string = "Flowtime")]
    FlowTime,
}

#[derive(Clone)]
pub enum TimeTechniquesSettingsValue {
    Duration(Duration),
    Int(u32),
}
impl Display for TimeTechniquesSettingsValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                TimeTechniquesSettingsValue::Duration(duration) => TimerWidget::format_time_delta(
                    chrono::Duration::from_std(*duration).unwrap_or_default(),
                ),
                TimeTechniquesSettingsValue::Int(n) => n.to_string(),
            }
        )
    }
}
pub struct TimeTechniquesSettingsEntry {
    pub name: String,
    pub value: TimeTechniquesSettingsValue,
    pub hint: String,
}
impl TimeTechniquesSettingsEntry {
    pub fn update(&self, input: &str) -> Result<Self> {
        debug!("New value input: {input}");
        let value = match self.value {
            TimeTechniquesSettingsValue::Duration(_) => TimeTechniquesSettingsValue::Duration(
                match NaiveTime::parse_from_str(input, "%H:%M:%S") {
                    Ok(t) => Ok(t),
                    Err(_) => NaiveTime::parse_from_str(&format!("0:{input}"), "%H:%M:%S"),
                }?
                .signed_duration_since(NaiveTime::default())
                .to_std()?,
            ),
            TimeTechniquesSettingsValue::Int(_) => {
                TimeTechniquesSettingsValue::Int(input.parse::<u32>()?)
            }
        };
        Ok(Self {
            name: self.name.clone(),
            value,
            hint: self.hint.clone(),
        })
    }
}
