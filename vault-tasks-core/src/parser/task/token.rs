use chrono::{NaiveDate, NaiveTime};

use crate::task::State;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Token {
    DueDate(NaiveDate),
    DueTime(NaiveTime),
    Name(String),
    Priority(usize),
    Tag(String),
    State(State),
    TodayFlag,
}
