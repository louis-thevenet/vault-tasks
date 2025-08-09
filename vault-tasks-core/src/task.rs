use color_eyre::{Result, eyre::bail};
use core::fmt;
use std::fmt::Write;
use std::{
    cmp::Ordering,
    fmt::Display,
    fs::{File, read_to_string},
    path::PathBuf,
};
use tracing::{debug, info};

use crate::TasksConfig;
use crate::config::{CoreConfig, PrettySymbolsConfig};

use super::date::Date;

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
    pub fn display(&self, state_symbols: &PrettySymbolsConfig) -> String {
        match self {
            Self::Done => state_symbols.task_done.clone(),
            Self::ToDo => state_symbols.task_todo.clone(),
            Self::Incomplete => state_symbols.task_incomplete.clone(),
            Self::Canceled => state_symbols.task_canceled.clone(),
        }
    }
}
impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let default_symbols = TasksConfig::default().pretty_symbols.clone();
        write!(f, "{}", self.display(&default_symbols))?;
        Ok(())
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct Task {
    pub subtasks: Vec<Task>,
    pub description: Option<String>,
    /// None means the task has no associated due date
    pub due_date: Option<Date>,
    pub filename: String,
    /// Line number in the file, if None then the task was not
    /// parsed from the file but added from CLI, it should be
    /// appended at the end.
    pub line_number: Option<usize>,
    pub name: String,
    pub priority: usize,
    pub completion: Option<usize>,
    pub state: State,
    pub tags: Option<Vec<String>>,
    pub is_today: bool,
}

impl Default for Task {
    fn default() -> Self {
        Self {
            due_date: None,
            name: String::new(),
            priority: 0,
            state: State::ToDo,
            tags: None,
            description: None,
            line_number: Some(1),
            subtasks: vec![],
            filename: String::new(),
            is_today: false,
            completion: None,
        }
    }
}

impl fmt::Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let default_symbols = PrettySymbolsConfig::default(); // TODO: that's bad, shouldn't use this default since it's empty!
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
        if let Some(date) = &self.due_date {
            write!(
                data_line,
                "{} {} ({}) ",
                default_symbols.due_date,
                date,
                date.get_relative_str()
            )?;
        }

        if self.priority > 0 {
            write!(data_line, "{}{} ", default_symbols.priority, self.priority)?;
        }
        if let Some(bar) = self.completion_bar_to_string(5, &default_symbols) {
            data_line.push_str(&bar);
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
    #[must_use]
    pub fn priority_to_string(&self, symbols: &PrettySymbolsConfig) -> String {
        if self.priority > 0 {
            format!("{}{}", symbols.priority, self.priority)
        } else {
            String::new()
        }
    }
    #[must_use]
    pub fn due_date_to_string(
        &self,
        symbols: &PrettySymbolsConfig,
        core_config: &CoreConfig,
    ) -> String {
        match &self.due_date {
            None => String::new(),
            Some(date) => {
                let this = &date;
                let due_date_symbol: &str = &symbols.due_date;
                format!(
                    "{due_date_symbol} {}",
                    this.to_string_format(core_config.use_american_format)
                )
            }
        }
    }
    #[must_use]
    pub fn is_today_to_string(&self, symbols: &PrettySymbolsConfig) -> String {
        if self.is_today {
            symbols.today_tag.to_string()
        } else {
            String::new()
        }
    }
    #[must_use]
    pub fn state_to_string(&self, symbols: &PrettySymbolsConfig) -> String {
        self.state.display(symbols)
    }
    #[must_use]
    pub fn completion_bar_to_string(
        &self,
        length: usize,
        symbols: &PrettySymbolsConfig,
    ) -> Option<String> {
        let percentage = self.completion?;
        let progress_bar = (1..=length)
            .map(|c| {
                if c * (100 / length) <= percentage {
                    symbols.progress_bar_true.clone()
                } else {
                    symbols.progress_bar_false.clone()
                }
            })
            .collect::<String>();

        Some(format!("[{progress_bar} {percentage}%]"))
    }
    pub fn get_fixed_attributes(&self, config: &TasksConfig, indent_length: usize) -> String {
        let indent = " ".repeat(indent_length);

        let state_str = match self.state {
            State::Done => config.task_state_markers.done,
            State::ToDo => config.task_state_markers.todo,
            State::Incomplete => config.task_state_markers.incomplete,
            State::Canceled => config.task_state_markers.canceled,
        };

        let priority = if self.priority > 0 {
            format!("p{}", self.priority)
        } else {
            String::new()
        };

        let completion = match self.completion {
            Some(c) => format!("c{c}"),
            None => String::new(),
        };

        let due_date = if let Some(due_date) = &self.due_date {
            due_date.to_string_format(config.core.use_american_format)
        } else {
            String::new()
        };

        let tags_str = self.tags.as_ref().map_or_else(String::new, |tags| {
            tags.clone()
                .iter()
                .map(|t| format!("#{t}"))
                .collect::<Vec<String>>()
                .join(" ")
        });

        let today_tag = if self.is_today {
            String::from("@today")
        } else {
            String::new()
        };

        let res = format!(
            "{}- [{}] {}",
            indent,
            state_str,
            [
                self.name.clone(),
                due_date,
                completion,
                priority,
                tags_str,
                today_tag
            ]
            .into_iter()
            .filter(|s| !String::is_empty(s))
            .collect::<Vec<String>>()
            .join(" ")
        );
        res.trim_end().to_string()
    }

    pub fn fix_task_attributes(&self, config: &TasksConfig, path: &PathBuf) -> Result<()> {
        if !path.is_file() {
            bail!("Tried to fix tasks attributes but {path:?} is not a file");
        }
        let content = read_to_string(path.clone())?;
        let mut lines = content
            .split('\n')
            .map(str::to_owned)
            .collect::<Vec<String>>();
        if let Some(line_number) = self.line_number {
            if lines.len() < line_number - 1 {
                bail!(
                    "Task's line number {} was greater than length of file {:?}",
                    line_number,
                    path
                );
            }

            let indent_length = lines[line_number - 1]
                .chars()
                .take_while(|c| c.is_whitespace())
                .count();

            let fixed_line = self.get_fixed_attributes(config, indent_length);

            if lines[line_number - 1] != fixed_line {
                debug!(
                    "\nReplacing\n{}\nWith\n{}\n",
                    lines[line_number - 1],
                    self.get_fixed_attributes(config, indent_length,)
                );
                lines[line_number - 1] = fixed_line;

                info!("Wrote to {path:?} at line {}", line_number);
            }
        } else {
            debug!("Creating a new task at end of file {}", path.display());
            let fixed_line = self.get_fixed_attributes(config, 0);
            lines.push(fixed_line);
            lines.push(String::new()); // Empty line
        }
        let mut file = File::create(path)?;
        std::io::Write::write_all(&mut file, lines.join("\n").as_bytes())?;

        Ok(())
    }
}

#[cfg(test)]
mod tests_tasks {
    use chrono::NaiveDate;
    use pretty_assertions::assert_eq;

    use crate::{
        TasksConfig,
        task::{Date, State, Task},
    };

    #[test]
    fn test_fix_attributes() {
        let config = TasksConfig {
            ..Default::default()
        };
        let task = Task {
            due_date: Some(Date::Day(NaiveDate::from_ymd_opt(2021, 12, 3).unwrap())),
            name: String::from("Test Task"),
            priority: 1,
            state: State::ToDo,
            tags: Some(vec![String::from("tag1"), String::from("tag2")]),
            description: Some(String::from("This is a test task.")),
            line_number: Some(2),
            ..Default::default()
        };
        let res = task.get_fixed_attributes(&config, 0);
        assert_eq!(res, "- [ ] Test Task 2021-12-03 p1 #tag1 #tag2");
    }

    #[test]
    fn test_fix_attributes_with_no_date() {
        let config = TasksConfig {
            ..Default::default()
        };
        let task = Task {
            due_date: None,
            name: String::from("Test Task with No Date"),
            priority: 2,
            state: State::Done,
            tags: Some(vec![String::from("tag3")]),
            description: None,
            line_number: Some(3),
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
            due_date: None,
            name: String::from("Test Task with Today tag"),
            priority: 2,
            state: State::Done,
            tags: Some(vec![String::from("tag3")]),
            description: None,
            line_number: Some(3),
            is_today: true,
            ..Default::default()
        };

        let res = task.get_fixed_attributes(&config, 0);
        assert_eq!(res, "- [x] Test Task with Today tag p2 #tag3 @today");
    }
}
#[cfg(test)]
mod tests_due_date {

    use crate::task::Date;
    use chrono::{Duration, Local};

    #[test]
    fn test_day_today() {
        let today = Local::now().naive_local().date();
        let due = Date::Day(today);
        assert_eq!(due.get_relative_str(), ("today".to_string()));
    }

    #[test]
    fn test_day_tomorrow() {
        let date = Local::now().naive_local().date() + Duration::days(1);
        let due = Date::Day(date);
        assert_eq!(due.get_relative_str(), ("tomorrow".to_string()));
    }

    #[test]
    fn test_day_yesterday() {
        let date = Local::now().naive_local().date() - Duration::days(1);
        let due = Date::Day(date);
        assert_eq!(due.get_relative_str(), ("yesterday".to_string()));
    }

    #[test]
    fn test_day_in_three_days() {
        let date = Local::now().naive_local().date() + Duration::days(3);
        let due = Date::Day(date);
        assert_eq!(due.get_relative_str(), ("in 3 days".to_string()));
    }

    #[test]
    fn test_day_two_weeks_ago() {
        let date = Local::now().naive_local().date() - Duration::weeks(2);
        let due = Date::Day(date);
        assert_eq!(due.get_relative_str(), ("2 weeks ago".to_string()));
    }

    #[test]
    fn test_day_in_two_months() {
        let date = Local::now().naive_local().date() + Duration::weeks(9); // ~2 months
        let due = Date::Day(date);
        assert_eq!(due.get_relative_str(), ("in 2 months".to_string()));
    }

    #[test]
    fn test_day_three_years_ago() {
        let date = Local::now().naive_local().date() - Duration::weeks(4 * 12 * 3);
        let due = Date::Day(date);
        assert_eq!(due.get_relative_str(), ("3 years ago".to_string()));
    }

    #[test]
    fn test_daytime_now() {
        let now = Local::now().naive_local();
        let due = Date::DayTime(now);
        assert_eq!(due.get_relative_str(), ("today".to_string()));
    }

    #[test]
    fn test_daytime_in_30_minutes() {
        let dt = Local::now().naive_local() + Duration::minutes(30);
        let due = Date::DayTime(dt);
        assert_eq!(due.get_relative_str(), ("in 30 minutes".to_string()));
    }

    #[test]
    fn test_daytime_three_hours_ago() {
        let dt = Local::now().naive_local() - Duration::hours(3);
        let due = Date::DayTime(dt);
        assert_eq!(due.get_relative_str(), ("3 hours ago".to_string()));
    }

    #[test]
    fn test_daytime_tomorrow_same_time() {
        let dt = Local::now().naive_local() + Duration::hours(24);
        let due = Date::DayTime(dt);
        assert_eq!(due.get_relative_str(), ("tomorrow".to_string()));
    }
    #[test]
    fn test_daytime_tomorrow_hours() {
        let dt = Local::now().naive_local() + Duration::hours(25);
        let due = Date::DayTime(dt);
        assert_eq!(due.get_relative_str(), ("in 25 hours".to_string()));
    }

    #[test]
    fn test_daytime_yesterday_same_time() {
        let dt = Local::now().naive_local() - Duration::hours(24);
        let due = Date::DayTime(dt);
        assert_eq!(due.get_relative_str(), ("yesterday".to_string()));
    }

    #[test]
    fn test_daytime_in_1_minute() {
        let dt = Local::now().naive_local() + Duration::minutes(1);
        let due = Date::DayTime(dt);
        assert_eq!(due.get_relative_str(), ("in 1 minutes".to_string()));
    }

    #[test]
    fn test_daytime_45_minutes_ago() {
        let dt = Local::now().naive_local() - Duration::minutes(45);
        let due = Date::DayTime(dt);
        assert_eq!(due.get_relative_str(), ("45 minutes ago".to_string()));
    }

    #[test]
    fn test_daytime_in_3_hours() {
        let dt = Local::now().naive_local() + Duration::hours(3);
        let due = Date::DayTime(dt);
        assert_eq!(due.get_relative_str(), ("in 3 hours".to_string()));
    }

    #[test]
    fn test_daytime_12_hours_ago() {
        let dt = Local::now().naive_local() - Duration::hours(12);
        let due = Date::DayTime(dt);
        assert_eq!(due.get_relative_str(), ("12 hours ago".to_string()));
    }

    #[test]
    fn test_daytime_in_1_day_exact() {
        let dt = Local::now().naive_local() + Duration::days(1);
        let due = Date::DayTime(dt);
        assert_eq!(due.get_relative_str(), ("tomorrow".to_string()));
    }

    #[test]
    fn test_daytime_exactly_yesterday() {
        let dt = Local::now().naive_local() - Duration::days(1);
        let due = Date::DayTime(dt);
        assert_eq!(due.get_relative_str(), ("yesterday".to_string()));
    }

    #[test]
    fn test_daytime_in_2_days() {
        let dt = Local::now().naive_local() + Duration::days(2);
        let due = Date::DayTime(dt);
        assert_eq!(due.get_relative_str(), ("in 2 days".to_string()));
    }

    #[test]
    fn test_daytime_10_days_ago() {
        let dt = Local::now().naive_local() - Duration::days(10);
        let due = Date::DayTime(dt);
        assert_eq!(due.get_relative_str(), ("10 days ago".to_string()));
    }

    #[test]
    fn test_daytime_in_3_weeks() {
        let dt = Local::now().naive_local() + Duration::weeks(3);
        let due = Date::DayTime(dt);
        assert_eq!(due.get_relative_str(), ("in 3 weeks".to_string()));
    }

    #[test]
    fn test_daytime_2_months_ago() {
        let dt = Local::now().naive_local() - Duration::weeks(9); // ≈ 2 months
        let due = Date::DayTime(dt);
        assert_eq!(due.get_relative_str(), ("2 months ago".to_string()));
    }
}
