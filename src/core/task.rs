use chrono::{NaiveDate, NaiveDateTime};
use color_eyre::{eyre::bail, Result};
use core::fmt;
use std::{
    cmp::Ordering,
    fmt::Display,
    fs::{read_to_string, File},
    io::Write,
    path::PathBuf,
};
use tracing::{debug, info};

use crate::core::{PrettySymbolsConfig, TasksConfig};

/// A task's state
/// Ordering is `Todo < Done`
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum State {
    ToDo,
    Done,
    Incomplete,
    Canceled,
}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (State::ToDo, State::ToDo)
            | (State::Done, State::Done)
            | (State::Canceled, State::Canceled)
            | (State::Incomplete, State::Incomplete) => Ordering::Equal,
            (State::Canceled | State::Done, State::ToDo)
            | (State::ToDo | State::Done | State::Canceled, State::Incomplete)
            | (State::Done, State::Canceled) => Ordering::Greater,
            (State::ToDo, State::Done | State::Canceled)
            | (State::Incomplete, State::ToDo | State::Done | State::Canceled)
            | (State::Canceled, State::Done) => Ordering::Less,
        }
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl State {
    pub fn display(&self, state_symbols: PrettySymbolsConfig) -> String {
        match self {
            Self::Done => state_symbols.task_done,
            Self::ToDo => state_symbols.task_todo,
            Self::Incomplete => state_symbols.task_incomplete,
            Self::Canceled => state_symbols.task_canceled,
        }
    }
}
impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let default_symbols = PrettySymbolsConfig::default();
        write!(f, "{}", self.display(default_symbols))?;
        Ok(())
    }
}
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
    pub fn to_display_format(&self, due_date_symbol: String, not_american_format: bool) -> String {
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
        let now = chrono::Local::now();
        let time_delta = match self {
            Self::NoDate => return None,
            Self::Day(date) => now.date_naive().signed_duration_since(*date),
            Self::DayTime(date_time) => {
                now.date_naive().signed_duration_since(date_time.date())
                    + now.time().signed_duration_since(date_time.time())
            }
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
        if time_delta.num_seconds() < 0 && time_delta_abs.num_days() == 1 {
            return Some(String::from("tomorrow"));
        }
        if time_delta.num_seconds() > 0 && time_delta_abs.num_days() == 1 {
            return Some(String::from("yesterday"));
        }

        let res = if 4 * 12 * 2 <= time_delta_abs.num_weeks() {
            format!("{} years", time_delta_abs.num_weeks() / (12 * 4))
        } else if 5 <= time_delta_abs.num_weeks() {
            format!("{} months", time_delta_abs.num_weeks() / 4)
        } else if 2 <= time_delta_abs.num_weeks() {
            format!("{} weeks", time_delta_abs.num_weeks())
        } else if 2 <= time_delta_abs.num_days() {
            format!("{} days", time_delta_abs.num_days())
        } else {
            format!("{} hours", time_delta_abs.num_hours())
        };
        Some(format!("{prefix}{res}{suffix}"))
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct Task {
    pub subtasks: Vec<Task>,
    pub description: Option<String>,
    pub due_date: DueDate,
    pub filename: String,
    pub line_number: usize,
    pub name: String,
    pub priority: usize,
    pub state: State,
    pub tags: Option<Vec<String>>,
    pub is_today: bool,
}

impl Default for Task {
    fn default() -> Self {
        Self {
            due_date: DueDate::NoDate,
            name: String::new(),
            priority: 0,
            state: State::ToDo,
            tags: None,
            description: None,
            line_number: 1,
            subtasks: vec![],
            filename: String::new(),
            is_today: false,
        }
    }
}

impl fmt::Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let default_symbols = PrettySymbolsConfig::default();
        let state = self.state.to_string();
        let title = format!("{state} {}", self.name);
        writeln!(f, "{title}")?;

        let mut data_line = String::new();
        let is_today = if self.is_today {
            format!("{} ", default_symbols.today_tag)
        } else {
            String::new()
        };
        data_line.push_str(&is_today);
        let due_date_str = self.due_date.to_string();

        if !due_date_str.is_empty() {
            data_line.push_str(&format!(
                "{} {due_date_str} ({})",
                default_symbols.due_date,
                self.due_date.get_relative_str().unwrap_or_default()
            ));
        }
        if self.priority > 0 {
            data_line.push_str(&format!("{}{} ", default_symbols.priority, self.priority));
        }
        if !data_line.is_empty() {
            writeln!(f, "{data_line}")?;
        }
        let mut tag_line = String::new();
        if self.tags.is_some() {
            tag_line.push_str(
                &self
                    .tags
                    .clone()
                    .unwrap()
                    .iter()
                    .map(|t| format!("#{t}"))
                    .collect::<Vec<String>>()
                    .join(" "),
            );
        }
        if !tag_line.is_empty() {
            writeln!(f, "{tag_line}")?;
        }
        if let Some(description) = self.description.clone() {
            for l in description.lines() {
                writeln!(f, "{l}")?;
            }
        }
        Ok(())
    }
}
impl Task {
    pub fn get_fixed_attributes(&self, config: &TasksConfig, indent_length: usize) -> String {
        let indent = " ".repeat(indent_length);

        let state_str = match self.state {
            State::Done => config.task_state_markers.done,
            State::ToDo => config.task_state_markers.todo,
            State::Incomplete => config.task_state_markers.incomplete,
            State::Canceled => config.task_state_markers.canceled,
        };

        let priority = if self.priority > 0 {
            format!("p{} ", self.priority)
        } else {
            String::new()
        };

        let mut due_date = self.due_date.to_string_format(!config.use_american_format);
        if !due_date.is_empty() {
            due_date.push(' ');
        }

        let tags_str = self.tags.as_ref().map_or_else(String::new, |tags| {
            tags.clone()
                .iter()
                .map(|t| format!("#{t}"))
                .collect::<Vec<String>>()
                .join(" ")
        });

        let today_tag = if self.is_today {
            String::from(" @today")
        } else {
            String::new()
        };

        let res = format!(
            "{}- [{}] {} {}{}{}{}",
            indent, state_str, self.name, due_date, priority, tags_str, today_tag
        );
        res.trim_end().to_string()
    }

    pub fn fix_task_attributes(&self, config: &TasksConfig, path: &PathBuf) -> Result<()> {
        let content = read_to_string(path.clone())?;
        let mut lines = content.split('\n').collect::<Vec<&str>>();

        if lines.len() < self.line_number - 1 {
            bail!(
                "Task's line number {} was greater than length of file {:?}",
                self.line_number,
                path
            );
        }

        let indent_length = lines[self.line_number - 1]
            .chars()
            .take_while(|c| c.is_whitespace())
            .count();

        let fixed_line = self.get_fixed_attributes(config, indent_length);

        if lines[self.line_number - 1] != fixed_line {
            debug!(
                "\nReplacing\n{}\nWith\n{}\n",
                lines[self.line_number - 1],
                self.get_fixed_attributes(config, indent_length,)
            );
            lines[self.line_number - 1] = &fixed_line;

            let mut file = File::create(path)?;
            file.write_all(lines.join("\n").as_bytes())?;

            info!("Wrote to {path:?} at line {}", self.line_number);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests_tasks {
    use chrono::NaiveDate;
    use pretty_assertions::assert_eq;

    use crate::core::{
        task::{DueDate, State, Task},
        TasksConfig,
    };

    #[test]
    fn test_fix_attributes() {
        let config = TasksConfig {
            use_american_format: true,
            ..Default::default()
        };
        let task = Task {
            due_date: DueDate::Day(NaiveDate::from_ymd_opt(2021, 12, 3).unwrap()),
            name: String::from("Test Task"),
            priority: 1,
            state: State::ToDo,
            tags: Some(vec![String::from("tag1"), String::from("tag2")]),
            description: Some(String::from("This is a test task.")),
            line_number: 2,
            ..Default::default()
        };
        let res = task.get_fixed_attributes(&config, 0);
        assert_eq!(res, "- [ ] Test Task 2021/12/03 p1 #tag1 #tag2");
    }

    #[test]
    fn test_fix_attributes_with_no_date() {
        let config = TasksConfig {
            ..Default::default()
        };
        let task = Task {
            due_date: DueDate::NoDate,
            name: String::from("Test Task with No Date"),
            priority: 2,
            state: State::Done,
            tags: Some(vec![String::from("tag3")]),
            description: None,
            line_number: 3,
            ..Default::default()
        };

        let res = task.get_fixed_attributes(&config, 0);
        assert_eq!(res, "- [x] Test Task with No Date p2 #tag3");
    }
    #[test]
    fn test_fix_attributes_with_today_tag() {
        let config = TasksConfig {
            ..Default::default()
        };
        let task = Task {
            due_date: DueDate::NoDate,
            name: String::from("Test Task with Today tag"),
            priority: 2,
            state: State::Done,
            tags: Some(vec![String::from("tag3")]),
            description: None,
            line_number: 3,
            is_today: true,
            ..Default::default()
        };

        let res = task.get_fixed_attributes(&config, 0);
        assert_eq!(res, "- [x] Test Task with Today tag p2 #tag3 @today");
    }
}
#[cfg(test)]
mod tests_due_date {
    use chrono::TimeDelta;

    use crate::core::task::DueDate;

    #[test]
    fn test_relative_date() {
        let now = chrono::Local::now();

        let tests = vec![
            (-1, "yesterday"),
            (0, "today"),
            (1, "tomorrow"),
            (7, "in 7 days"),
            (17, "in 2 weeks"),
            (65, "in 2 months"),
            (800, "in 2 years"),
        ];
        for (days, res) in tests {
            let due_date = DueDate::Day(
                now.checked_add_signed(TimeDelta::days(days))
                    .unwrap()
                    .date_naive(),
            );
            assert_eq!(due_date.get_relative_str(), Some(String::from(res)));
        }
    }
}
