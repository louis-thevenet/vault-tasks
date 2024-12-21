use color_eyre::{eyre::bail, Result};
use std::time::Duration;

use crate::time_management::{time_management_technique::TimeManagementTechnique, State};

#[derive(Debug)]
pub struct FlowTime {
    break_factor: u32,
    break_time_excess: f32,
}

impl FlowTime {
    /// Creates a new `FlowTime` object from a break time factor.
    /// After the first focus time (t), break time will be computed as t / `break_factor`
    /// # Errors
    /// Will return an error if `break_factor` <= 0
    pub fn new(break_factor: u32) -> Result<Self> {
        if break_factor == 0 {
            bail!("Break Factor for FlowTime is negative")
        }
        Ok(Self {
            break_factor,
            break_time_excess: 0_f32,
        })
    }
}

impl TimeManagementTechnique for FlowTime {
    fn switch(&mut self, state: &Option<State>, time_spent: Duration) -> State {
        match state {
            Some(State::Focus(_)) => State::Break(Some(
                time_spent / self.break_factor + Duration::from_secs_f32(self.break_time_excess),
            )),
            Some(State::Break(Some(time_to_spend))) => {
                let delta = time_to_spend.as_secs_f32() - time_spent.as_secs_f32();
                self.break_time_excess = 0_f32.max(delta); // save break time excess
                State::Focus(None)
            }
            Some(State::Break(None)) | None => State::Focus(None),
        }
    }
}
