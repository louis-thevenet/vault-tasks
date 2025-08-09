use std::{fmt::Debug, time::Duration};

use super::State;

pub trait TimeManagementTechnique: Debug {
    fn change_state_or_freeze(
        &self,
        auto_skip: bool,
        from_clock: bool,
        next_state: State,
    ) -> State {
        if auto_skip || !from_clock {
            next_state
        } else {
            State::Frozen(Box::new(next_state))
        }
    }
    fn switch(&mut self, state: &Option<State>, from_clock: bool, time_spent: Duration) -> State;
}
