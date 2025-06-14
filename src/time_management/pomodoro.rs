use core::panic;
use std::time::Duration;

use crate::time_management::{State, time_management_technique::TimeManagementTechnique};

#[derive(Debug, PartialEq, PartialOrd)]
pub struct Pomodoro {
    auto_skip: bool,
    focus_duration: Duration,
    break_count: usize,
    short_breaks_before_long: usize,
    short_break_duration: Duration,
    long_break_duration: Duration,
    focus_time_excess: f32,
    break_time_excess: f32,
}
impl Pomodoro {
    pub fn new(
        auto_skip: bool,
        focus_duration: Duration,
        short_breaks_before_long: usize,
        short_break_duration: Duration,
        long_break_duration: Duration,
    ) -> Self {
        Self {
            auto_skip,
            focus_duration,
            break_count: 0,
            short_breaks_before_long,
            short_break_duration,
            long_break_duration,
            break_time_excess: 0_f32,
            focus_time_excess: 0_f32,
        }
    }

    pub fn classic_pomodoro() -> Self {
        Self {
            auto_skip: true,
            focus_duration: Duration::from_secs(25 * 60),
            short_break_duration: Duration::from_secs(5 * 60),
            long_break_duration: Duration::from_secs(15 * 60),
            break_count: 0,
            short_breaks_before_long: 3,
            break_time_excess: 0_f32,
            focus_time_excess: 0_f32,
        }
    }
}
impl TimeManagementTechnique for Pomodoro {
    fn switch(&mut self, state: &Option<State>, from_clock: bool, time_spent: Duration) -> State {
        match state {
            Some(State::Frozen(next_state)) => *next_state.clone(),
            Some(State::Focus(None)) => panic!("invalid state"),

            Some(State::Focus(Some(time_to_spend))) => {
                let delta = time_to_spend.as_secs_f32() - time_spent.as_secs_f32();
                self.focus_time_excess = 0_f32.max(delta);
                let res = if self.short_breaks_before_long == self.break_count {
                    self.break_count = 0;
                    State::Break(Some(
                        self.long_break_duration + Duration::from_secs_f32(self.break_time_excess),
                    ))
                } else {
                    self.break_count += 1;
                    State::Break(Some(
                        self.short_break_duration + Duration::from_secs_f32(self.break_time_excess),
                    ))
                };
                self.change_state_or_freeze(self.auto_skip, from_clock, res)
            }

            Some(State::Break(Some(time_to_spend))) => {
                let delta = time_to_spend.as_secs_f32() - time_spent.as_secs_f32();
                self.break_time_excess = 0_f32.max(delta);
                let res = State::Focus(Some(
                    self.focus_duration + Duration::from_secs_f32(self.focus_time_excess),
                ));
                self.change_state_or_freeze(self.auto_skip, from_clock, res)
            }
            Some(State::Break(None)) | None => State::Focus(Some(self.focus_duration)),
        }
    }
}
