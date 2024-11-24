use chrono::NaiveTime;
use color_eyre::Result;
use std::{fmt::Display, time::Duration};
use strum::{EnumIter, FromRepr};

use tracing::debug;

use crate::widgets::timer::TimerWidget;

#[derive(Default, Clone, Copy, FromRepr, EnumIter, strum_macros::Display, PartialEq, Eq, Hash)]
pub enum MethodsAvailable {
    #[default]
    #[strum(to_string = "Pomodoro")]
    Pomodoro,
    #[strum(to_string = "Flowtime")]
    FlowTime,
}

#[derive(Clone)]
/// Represents every value a method setting can be.
pub enum MethodSettingsValue {
    Duration(Duration),
    Int(u32),
}
impl Display for MethodSettingsValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                MethodSettingsValue::Duration(duration) => TimerWidget::format_time_delta(
                    chrono::Duration::from_std(*duration).unwrap_or_default(),
                ),
                MethodSettingsValue::Int(n) => n.to_string(),
            }
        )
    }
}

/// Represents an entry in the setting table of a method.
pub struct MethodSettingsEntry {
    /// Name of the setting
    pub name: String,
    /// Setting value
    pub value: MethodSettingsValue,
    /// An hint on the setting
    pub hint: String,
}
impl MethodSettingsEntry {
    /// Parses an input string to a `MethodSettingValue`
    pub fn update(&self, input: &str) -> Result<Self> {
        debug!("New value input: {input}");
        let value = match self.value {
            MethodSettingsValue::Duration(_) => MethodSettingsValue::Duration(
                match NaiveTime::parse_from_str(input, "%H:%M:%S") {
                    Ok(t) => Ok(t),
                    Err(_) => NaiveTime::parse_from_str(&format!("0:{input}"), "%H:%M:%S"),
                }?
                .signed_duration_since(NaiveTime::default())
                .to_std()?,
            ),
            MethodSettingsValue::Int(_) => MethodSettingsValue::Int(input.parse::<u32>()?),
        };
        Ok(Self {
            name: self.name.clone(),
            value,
            hint: self.hint.clone(),
        })
    }
}
