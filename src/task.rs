use chrono::{NaiveDate, NaiveDateTime};
use core::fmt;
use std::fmt::Display;

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum State {
    Done,
    ToDo,
}
impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Done => write!(f, "Done")?,
            Self::ToDo => write!(f, "To Do")?,
        }
        Ok(())
    }
}
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
/// This type accounts for the case where the task has a due date but no exact due time
pub enum DueDate {
    Day(NaiveDate),
    DayTime(NaiveDateTime),
}
impl Display for DueDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Day(date) => write!(f, "{date}"),
            Self::DayTime(date) => write!(f, "{date}"),
        }
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct Task {
    pub due_date: Option<DueDate>,
    pub name: String,
    pub priority: usize,
    pub state: State,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
}

impl Default for Task {
    fn default() -> Self {
        let now = chrono::Local::now();
        let due_date = Some(DueDate::Day(now.date_naive()));
        Self {
            due_date,
            name: String::from("New Task"),
            priority: 0,
            state: State::ToDo,
            tags: None,
            description: None,
        }
    }
}

impl fmt::Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Name: \t -> {0}", self.name)?;
        if let Some(due_date) = &self.due_date {
            writeln!(f, "Due: \t -> {due_date}")?;
        }
        writeln!(f, "State: \t -> {}", self.state)?;
        writeln!(f, "Prio.: \t -> {}", self.priority)?;

        if let Some(description) = &self.description {
            writeln!(f, "Desc.:\t -> \"{description}\"")?;
        }
        if let Some(tags) = &self.tags {
            writeln!(f, "Tags: \t -> {tags:?}")?;
        }
        fmt::Result::Ok(())
    }
}
