use chrono::{NaiveDate, NaiveTime};

use crate::task::TaskState;

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    DueDate(NaiveDate),
    DueTime(NaiveTime),
    Name(String),
    Priority(usize),
    Tag(String),
    State(TaskState),
}
