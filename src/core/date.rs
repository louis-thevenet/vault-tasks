use std::{cmp::Ordering, fmt::Display};

use chrono::{NaiveDate, NaiveDateTime, Timelike};

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
/// This type accounts for the case where the task has a due date but no exact due time
pub enum DueDate {
    NoDate,
    Day(NaiveDate),
    DayTime(NaiveDateTime),
}
impl Display for DueDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Day(date) => write!(f, "{date}"),
            Self::DayTime(date) => write!(f, "{date}"),
            Self::NoDate => Ok(()),
        }
    }
}

impl DueDate {
    #[must_use]
    pub fn to_display_format(&self, due_date_symbol: &str, not_american_format: bool) -> String {
        if matches!(self, Self::NoDate) {
            String::new()
        } else {
            format!(
                "{due_date_symbol} {}",
                self.to_string_format(not_american_format)
            )
        }
    }
    #[must_use]
    pub fn to_string_format(&self, not_american_format: bool) -> String {
        let format_date = if not_american_format {
            "%d/%m/%Y"
        } else {
            "%Y/%m/%d"
        };
        let format_datetime = if not_american_format {
            "%d/%m/%Y %T"
        } else {
            "%Y/%m/%d %T"
        };

        match self {
            Self::Day(date) => date.format(format_date).to_string(),
            Self::DayTime(date) => date.format(format_datetime).to_string(),
            Self::NoDate => String::new(),
        }
    }

    #[must_use]
    pub fn get_relative_str(&self) -> Option<String> {
        // This truncation prevents errors such as 23:59:59:999... instead of 24 hours
        let now = chrono::Local::now()
            .with_second(0)
            .unwrap()
            .with_nanosecond(0)
            .unwrap();

        let time_delta = match self {
            DueDate::Day(naive_date) => now.date_naive().signed_duration_since(*naive_date),
            DueDate::DayTime(naive_date_time) => {
                now.date_naive()
                    .signed_duration_since(naive_date_time.date())
                    // same truncation here
                    + now.time().signed_duration_since(
                        naive_date_time
                            .time()
                            .with_second(0)
                            .unwrap()
                            .with_nanosecond(0)
                            .unwrap(),
                    )
            }
            DueDate::NoDate => return None,
        };
        let (prefix, suffix) = match time_delta.num_seconds().cmp(&0) {
            Ordering::Less => (String::from("in "), String::new()),
            Ordering::Equal => (String::new(), String::new()),
            Ordering::Greater => (String::new(), String::from(" ago")),
        };

        let time_delta_abs = time_delta.abs();

        if time_delta_abs.is_zero() {
            return Some(String::from("today"));
        }
        if time_delta.num_seconds() < 0 && time_delta_abs.num_hours() == 24 {
            return Some(String::from("tomorrow"));
        }
        if time_delta.num_seconds() > 0 && time_delta_abs.num_hours() == 24 {
            return Some(String::from("yesterday"));
        }

        // >= 13 months -> show years
        let res = if 4 * 12 < time_delta_abs.num_weeks() {
            format!("{} years", time_delta_abs.num_weeks() / (12 * 4))
            // >= 5 weeks -> show months
        } else if 5 <= time_delta_abs.num_weeks() {
            format!("{} months", time_delta_abs.num_weeks() / 4)
            // >= 2 weeks -> show weeks
        } else if 2 <= time_delta_abs.num_weeks() {
            format!("{} weeks", time_delta_abs.num_weeks())
            // >= 2 days -> show days
        } else if 2 <= time_delta_abs.num_days() {
            format!("{} days", time_delta_abs.num_days())
            // >= 2 hours -> show hours
        } else if 2 <= time_delta_abs.num_hours() {
            format!("{} hours", time_delta_abs.num_hours())
        } else {
            format!("{} minutes", time_delta_abs.num_minutes())
        };
        Some(format!("{prefix}{res}{suffix}"))
    }
}
