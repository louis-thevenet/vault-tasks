use std::{fmt::Debug, time::Duration};

use super::State;

pub trait TimeManagementTechnique: Debug {
    fn switch(&mut self, state: &Option<State>, time_spent: Duration) -> State;
}
