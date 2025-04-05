use std::time::Duration;

use pomodoro::Pomodoro;
use time_management_technique::TimeManagementTechnique;
use tracing::debug;

pub mod flow_time;
pub mod pomodoro;
pub mod time_management_technique;

#[derive(Debug, PartialEq, Clone)]
pub enum State {
    Frozen(Box<State>),
    Focus(Option<Duration>),
    Break(Option<Duration>),
}
#[derive(Debug)]
/// Provides tracking methods using a generic `TimeTrackingTechnique`
pub struct TimeManagementEngine {
    pub mode: Box<dyn TimeManagementTechnique>,
    pub state: Option<State>,
}
impl Default for TimeManagementEngine {
    fn default() -> Self {
        Self {
            mode: Box::new(Pomodoro::classic_pomodoro()),
            state: None,
        }
    }
}
impl TimeManagementEngine {
    /// Creates a new [`TimeTrackingEngine<T>`].
    pub fn new(technique: Box<dyn TimeManagementTechnique>) -> Self {
        Self {
            mode: technique,
            state: None,
        }
    }

    /// Returns the next state of the time tracking engine.
    /// # Argument
    /// - `from_clock`: bool : Whether the switch was triggered automatically or not.
    /// - `time_spent: Duration`: The duration of the previous session.
    /// # Returns
    /// - `Option<Duration>`: Whether there is or not an explicit duration for the next session
    /// - `TimeManagementEngine<T>`: The next state of the engine
    pub fn switch(&mut self, from_clock: bool, time_spent: Duration) -> State {
        let new_state = self.mode.switch(&self.state, from_clock, time_spent);
        debug!("{:?}", new_state);
        self.state = Some(new_state.clone());
        new_state
    }
}

#[cfg(test)]
mod tests {
    use color_eyre::eyre::Result;

    use crate::time_management::{
        flow_time::FlowTime, pomodoro::Pomodoro, State, TimeManagementEngine,
    };

    use std::time::Duration;

    #[test]
    fn test_run_pomodoro() {
        let mut time_tracker = TimeManagementEngine::new(Box::new(Pomodoro::classic_pomodoro()));
        let focus_time = Duration::from_secs(60 * 25);
        let short_break_time = Duration::from_secs(60 * 5);
        assert!(time_tracker.state.is_none());

        let to_spend_opt = time_tracker.switch(true, Duration::default());
        assert!(time_tracker.state.is_some());
        assert_eq!(
            time_tracker.state.clone().unwrap(),
            State::Focus(Some(focus_time))
        );
        assert_eq!(State::Focus(Some(focus_time)), to_spend_opt);

        let to_spend_opt = time_tracker.switch(true, Duration::default());
        assert!(time_tracker.state.is_some());
        assert_eq!(
            time_tracker.state.clone().unwrap(),
            State::Break(Some(short_break_time))
        );
        assert_eq!(State::Break(Some(short_break_time)), to_spend_opt);
    }
    #[test]
    fn test_full_run_pomodoro() {
        let mut time_tracker = TimeManagementEngine::new(Box::new(Pomodoro::classic_pomodoro()));
        assert!(time_tracker.state.is_none());

        let mut to_spend_opt = State::Focus(None);

        for _i in 0..2 {
            // (Focus -> Break) 3 times
            for _j in 0..(3 * 2) {
                let to_spend_opt2 = time_tracker.switch(true, Duration::from_secs(0));
                to_spend_opt = to_spend_opt2;
            }

            assert!(time_tracker.state.is_some());
            assert_eq!(time_tracker.state.clone().unwrap(), to_spend_opt);
        }
    }
    #[test]
    fn test_full_run_pomodoro_manual_skip() {
        let technique = Pomodoro::new(false, Duration::ZERO, 4, Duration::ZERO, Duration::ZERO);

        let mut time_tracker = TimeManagementEngine::new(Box::new(technique));
        assert!(time_tracker.state.is_none());

        let mut to_spend_opt = State::Focus(None);

        for _i in 0..2 {
            // (Focus -> Break) 3 times
            for _j in 0..(3 * 2) {
                time_tracker.switch(true, Duration::from_secs(0));
                let to_spend_opt2 = time_tracker.switch(true, Duration::ZERO); // actually skip
                to_spend_opt = to_spend_opt2;
            }

            assert!(time_tracker.state.is_some());
            assert_eq!(time_tracker.state.clone().unwrap(), to_spend_opt);
        }
    }
    #[test]
    fn test_run_flowtime() -> Result<()> {
        let break_factor = 5;
        let mut time_tracker =
            TimeManagementEngine::new(Box::new(FlowTime::new(true, break_factor)?));

        assert!(time_tracker.state.is_none());

        let focus_time = Duration::from_secs(25);
        let break_time = focus_time / break_factor;

        let to_spend_opt = time_tracker.switch(true, Duration::from_secs(0));

        assert_eq!(State::Focus(None), to_spend_opt);
        assert!(time_tracker.state.is_some());
        assert_eq!(time_tracker.state.clone().unwrap(), State::Focus(None));

        let to_spend_opt = time_tracker.switch(true, focus_time);
        assert!(time_tracker.state.is_some());
        assert_eq!(
            time_tracker.state.clone().unwrap(),
            State::Break(Some(break_time))
        );
        assert_eq!(State::Break(Some(break_time)), to_spend_opt);
        Ok(())
    }
    #[test]
    fn test_run_flowtime_excess_break_time() -> Result<()> {
        let break_factor = 5;
        let mut time_tracker =
            TimeManagementEngine::new(Box::new(FlowTime::new(true, break_factor)?));

        assert!(time_tracker.state.is_none());

        let focus_time = Duration::from_secs(25);
        let break_time = focus_time / break_factor;

        let to_spend_opt = time_tracker.switch(true, Duration::from_secs(0));

        assert_eq!(State::Focus(None), to_spend_opt);
        assert!(time_tracker.state.is_some());
        assert_eq!(time_tracker.state.clone().unwrap(), State::Focus(None));

        let to_spend_opt = time_tracker.switch(true, focus_time);
        assert!(time_tracker.state.is_some());
        assert_eq!(
            time_tracker.state.clone().unwrap(),
            State::Break(Some(break_time))
        );
        assert_eq!(State::Break(Some(break_time)), to_spend_opt);

        // Break time lasted 2s instead of 5s
        let break_time_skipped = Duration::from_secs(3);
        time_tracker.switch(true, break_time - break_time_skipped);
        let to_spend_opt = time_tracker.switch(true, focus_time);
        assert!(time_tracker.state.is_some());
        assert_eq!(
            time_tracker.state.clone().unwrap(),
            State::Break(Some(break_time + break_time_skipped))
        );
        assert_eq!(
            State::Break(Some(break_time + break_time_skipped)),
            to_spend_opt
        );

        // Ensures we return to normal cycle
        let to_spend_opt = time_tracker.switch(true, break_time + break_time_skipped);
        assert_eq!(State::Focus(None), to_spend_opt);
        assert!(time_tracker.state.is_some());
        assert_eq!(time_tracker.state.clone().unwrap(), State::Focus(None));

        // Break time is normal
        let to_spend_opt = time_tracker.switch(true, focus_time);
        assert!(time_tracker.state.is_some());
        assert_eq!(
            time_tracker.state.clone().unwrap(),
            State::Break(Some(break_time))
        );
        assert_eq!(State::Break(Some(break_time)), to_spend_opt);
        Ok(())
    }
    #[test]
    fn test_run_pomodoro_excess_break_time() {
        let mut time_tracker = TimeManagementEngine::new(Box::new(Pomodoro::classic_pomodoro()));

        assert!(time_tracker.state.is_none());

        let focus_time = Duration::from_secs(1500);
        let break_time = Duration::from_secs(1500 / 5);

        // Init -> Focus
        let to_spend_opt = time_tracker.switch(true, Duration::from_secs(0));

        assert_eq!(State::Focus(Some(focus_time)), to_spend_opt);
        assert!(time_tracker.state.is_some());
        assert_eq!(
            time_tracker.state.clone().unwrap(),
            State::Focus(Some(focus_time))
        );

        // Focus -> Break
        let to_spend_opt = time_tracker.switch(true, focus_time);
        assert!(time_tracker.state.is_some());
        assert_eq!(
            time_tracker.state.clone().unwrap(),
            State::Break(Some(break_time))
        );
        assert_eq!(State::Break(Some(break_time)), to_spend_opt);

        // Break skipped early -> Focus
        let break_time_skipped = Duration::from_secs(3);
        time_tracker.switch(true, break_time - break_time_skipped);

        // Focus skipped early -> Break
        let focus_time_skipped = Duration::from_secs(9);
        let to_spend_opt = time_tracker.switch(true, focus_time - focus_time_skipped);

        // Is break time extended ?
        assert!(time_tracker.state.is_some());
        assert_eq!(
            time_tracker.state.clone().unwrap(),
            State::Break(Some(break_time + break_time_skipped))
        );
        assert_eq!(
            State::Break(Some(break_time + break_time_skipped)),
            to_spend_opt
        );

        // Break -> Focus
        let to_spend_opt = time_tracker.switch(true, break_time + break_time_skipped);

        // Is focus time extended ?
        assert_eq!(
            State::Focus(Some(focus_time + focus_time_skipped)),
            to_spend_opt
        );
        assert!(time_tracker.state.is_some());
        assert_eq!(
            time_tracker.state.clone().unwrap(),
            State::Focus(Some(focus_time + focus_time_skipped))
        );

        // Break time is normal
        let to_spend_opt = time_tracker.switch(true, focus_time + focus_time_skipped);
        assert!(time_tracker.state.is_some());
        assert_eq!(
            time_tracker.state.clone().unwrap(),
            State::Break(Some(break_time))
        );
        assert_eq!(State::Break(Some(break_time)), to_spend_opt);

        // Focus time is normal
        let to_spend_opt = time_tracker.switch(true, break_time);
        assert!(time_tracker.state.is_some());
        assert_eq!(
            time_tracker.state.clone().unwrap(),
            State::Focus(Some(focus_time))
        );
        assert_eq!(State::Focus(Some(focus_time)), to_spend_opt);
    }
}
