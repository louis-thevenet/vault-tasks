use crate::{config::Config, scanner::Scanner, task::Task};
use anyhow::Result;
use std::collections::HashSet;

pub struct TaskManager {
    tasks: HashSet<Task>,
}
impl TaskManager {
    pub fn load_from_config(config: Config) -> Result<Self> {
        let scanner = Scanner::new(config);
        let tasks = scanner.scan_vault()?;
        Ok(Self { tasks })
    }
}
