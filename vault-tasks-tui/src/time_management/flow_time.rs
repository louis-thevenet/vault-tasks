use color_eyre::{Result, eyre::bail};
use std::time::Duration;

use crate::time_management::{State, time_management_technique::TimeManagementTechnique};

#[derive(Debug)]
pub struct FlowTime {
    auto_skip: bool,
    break_factor: u32,
    break_time_excess: f32,
}

impl FlowTime {
    /// Creates a new `FlowTime` object from a break time factor.
    /// After the first focus time (t), break time will be computed as t / `break_factor`
    /// # Errors
    /// Will return an error if `break_factor` <= 0
    pub fn new(auto_skip: bool, break_factor: u32) -> Result<Self> {
        if break_factor == 0 {
            bail!("Break Factor for FlowTime is negative")
        }
        Ok(Self {
            auto_skip,
            break_factor,
            break_time_excess: 0_f32,
        })
    }
}

impl TimeManagementTechnique for FlowTime {
    fn switch(&mut self, state: &Option<State>, from_clock: bool, time_spent: Duration) -> State {
        match state {
            Some(State::Frozen(next_state)) => *next_state.clone(),
            Some(State::Focus(_)) => self.change_state_or_freeze(
                self.auto_skip,
                from_clock,
                State::Break(Some(
                    time_spent / self.break_factor
                        + Duration::from_secs_f32(self.break_time_excess),
                )),
            ),

            Some(State::Break(Some(time_to_spend))) => {
                let delta = time_to_spend.as_secs_f32() - time_spent.as_secs_f32();
                self.break_time_excess = 0_f32.max(delta); // save break time excess
                self.change_state_or_freeze(self.auto_skip, from_clock, State::Focus(None))
            }
            Some(State::Break(None)) | None => State::Focus(None),
        }
    }
}
