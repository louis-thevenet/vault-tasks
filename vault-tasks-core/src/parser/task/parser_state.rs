use winnow::{
    Parser, Result,
    combinator::{delimited, preceded},
    token::any,
};

use crate::config::TaskMarkerConfig;
use crate::task::State;

use super::token::Token;

/// Parses a `TaskState` from an input string.
pub fn parse_task_state(input: &mut &str, task_marker_config: &TaskMarkerConfig) -> Result<Token> {
    match preceded("- ", delimited("[", any, "]")).parse_next(input) {
        Ok(c) => {
            if c == task_marker_config.todo {
                Ok(Token::State(State::ToDo))
            } else if c == task_marker_config.incomplete {
                Ok(Token::State(State::Incomplete))
            } else if c == task_marker_config.canceled {
                Ok(Token::State(State::Canceled))
            } else {
                Ok(Token::State(State::Done))
            }
        }

        Err(error) => Err(error),
    }

    // // This version only supports X to mark tasks done
    // match alt(("- [ ]", "- [X]")).parse_next(input) {
    //     Err(error) => Err(error),
    //     Ok("- [ ]") => Ok(Token::State(TaskState::ToDo)),
    //     _ => Ok(Token::State(TaskState::Done)),
    // }
}
#[cfg(test)]
mod test {
    use crate::{
        config::TaskMarkerConfig,
        parser::task::{parser_state::parse_task_state, token::Token},
        task::State,
    };
    fn config() -> TaskMarkerConfig {
        TaskMarkerConfig {
            done: 'x',
            todo: ' ',
            incomplete: '/',
            canceled: '-',
        }
    }

    #[test]
    fn test_parse_task_state_todo() {
        let mut input = "- [ ]";
        let expected = Ok(Token::State(State::ToDo));
        let config = &config();
        assert_eq!(parse_task_state(&mut input, config), expected);
    }
    #[test]
    fn test_parse_task_state_done() {
        let mut input = "- [X]";
        let expected = Ok(Token::State(State::Done));
        let config = &config();
        assert_eq!(parse_task_state(&mut input, config), expected);
    }
    #[test]
    fn test_parse_task_state_done_alt() {
        let mut input = "- [o]";
        let expected = Ok(Token::State(State::Done));
        let config = &config();
        assert_eq!(parse_task_state(&mut input, config), expected);
    }
    #[test]
    fn test_parse_task_state_canceled() {
        let mut input = "- [-]";
        let expected = Ok(Token::State(State::Canceled));
        let config = &config();
        assert_eq!(parse_task_state(&mut input, config), expected);
    }
    #[test]
    fn test_parse_task_state_fail() {
        let mut input = "- o]";
        let config = &config();
        assert!(parse_task_state(&mut input, config).is_err());
    }
}
