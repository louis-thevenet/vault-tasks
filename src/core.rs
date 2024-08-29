use anyhow::Result;
use chrono::NaiveDateTime;
use core::fmt;
use std::collections::HashSet;

use crate::{config::Config, scanner::Scanner};
#[derive(Hash, Eq, PartialEq, Clone)]
pub struct Task {
    pub due_date: NaiveDateTime,
    pub name: String,
}
impl fmt::Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Task \t -> {0}", self.name)?;
        writeln!(
            f,
            "Due on \t -> {0}",
            self.due_date.format("%A %d/%m, %Hh%M")
        )?;
        fmt::Result::Ok(())
    }
}
impl fmt::Debug for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}, due on {}", self.name, self.due_date)?;
        fmt::Result::Ok(())
    }
}

pub struct TaskManager {
    tasks: HashSet<Task>,
}
impl TaskManager {
    pub fn new(config: Config) -> Result<Self> {
        let scanner = Scanner::new(config);
        let tasks = scanner.scan_vault()?;
        Ok(TaskManager { tasks })
    }
}
