use chrono::{NaiveDate, NaiveDateTime};
use color_eyre::{eyre::bail, Result};
use core::fmt;
use std::{
    fmt::Display,
    fs::{read_to_string, File},
    io::Write,
    path::PathBuf,
};
use tracing::{debug, info};

use crate::config::Config;

const STATE_TO_DO_EMOJI: &str = "❌";
const STATE_DONE_EMOJI: &str = "✅";
pub const DUE_DATE_EMOJI: &str = "📅";
pub const PRIORITY_EMOJI: &str = "❗";
pub const TODAY_FLAG_EMOJI: &str = "☀️";
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum State {
    Done,
    ToDo,
}
impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Done => write!(f, "{STATE_DONE_EMOJI}")?,
            Self::ToDo => write!(f, "{STATE_TO_DO_EMOJI}")?,
        }
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
    pub fn to_display_format(&self, not_american_format: bool) -> String {
        if matches!(self, Self::NoDate) {
            String::new()
        } else {
            format!(
                "{DUE_DATE_EMOJI} {}",
                self.to_string_format(not_american_format)
            )
        }
    }
    pub fn to_string_format(&self, not_american_format: bool) -> String {
        let format_date = if !not_american_format {
            "%Y/%m/%d"
        } else {
            "%d/%m/%Y"
        };
        let format_datetime = if !not_american_format {
            "%Y/%m/%d %T"
        } else {
            "%d/%m/%Y %T"
        };

        match self {
            Self::Day(date) => date.format(format_date).to_string(),
            Self::DayTime(date) => date.format(format_datetime).to_string(),
            Self::NoDate => String::new(),
        }
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
        let state = self.state.to_string();
        let title = format!("{state} {}", self.name);
        writeln!(f, "{title}")?;

        let mut data_line = String::new();
        let is_today = if self.is_today {
            format!("{TODAY_FLAG_EMOJI} ")
        } else {
            String::new()
        };
        data_line.push_str(&is_today);
        let due_date_str = self.due_date.to_string();

        if !due_date_str.is_empty() {
            data_line.push_str(&format!("{DUE_DATE_EMOJI} {due_date_str} "));
        }
        if self.priority > 0 {
            data_line.push_str(&format!("{}{} ", PRIORITY_EMOJI, self.priority));
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
    fn get_fixed_attributes(&self, config: &Config, indent_length: usize) -> String {
        let indent = " ".repeat(indent_length);

        let state_str = match self.state {
            State::Done => "X",
            State::ToDo => " ",
        };

        let priority = if self.priority > 0 {
            format!("p{} ", self.priority)
        } else {
            String::new()
        };

        let mut due_date = self
            .due_date
            .to_string_format(!config.tasks_config.use_american_format);
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

    pub fn fix_task_attributes(&self, config: &Config, path: &PathBuf) -> Result<()> {
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
mod tests {
    use chrono::NaiveDate;

    use crate::{
        config::Config,
        task_core::task::{DueDate, State, Task},
    };

    #[test]
    fn test_fix_attributes() {
        let mut config = Config::default();
        config.tasks_config.use_american_format = true;
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
        let config = Config::default();
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
        assert_eq!(res, "- [X] Test Task with No Date p2 #tag3");
    }
    #[test]
    fn test_fix_attributes_with_today_tag() {
        let config = Config::default();
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
        assert_eq!(res, "- [X] Test Task with Today tag p2 #tag3 @today");
    }
}
