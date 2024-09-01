use winnow::{
    combinator::{delimited, preceded},
    token::any,
    PResult, Parser,
};

use crate::core::TaskState;

use super::token::Token;
/// Parses a `TaskState` from an input string.
pub fn parse_task_state(input: &mut &str) -> PResult<Token> {
    match preceded("- ", delimited("[", any, "]")).parse_next(input) {
        Ok(' ') => Ok(Token::State(TaskState::ToDo)),
        Ok(_) => Ok(Token::State(TaskState::Done)),
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
        core::TaskState,
        parser::{parser_state::parse_task_state, token::Token},
    };

    #[test]
    fn test_parse_task_state_todo() {
        let mut input = "- [ ]";
        let expected = Ok(Token::State(TaskState::ToDo));
        assert_eq!(parse_task_state(&mut input), expected);
    }
    #[test]
    fn test_parse_task_state_done() {
        let mut input = "- [X]";
        let expected = Ok(Token::State(TaskState::Done));
        assert_eq!(parse_task_state(&mut input), expected);
    }
    #[test]
    fn test_parse_task_state_done_alt() {
        let mut input = "- [o]";
        let expected = Ok(Token::State(TaskState::Done));
        assert_eq!(parse_task_state(&mut input), expected);
    }
    #[test]
    fn test_parse_task_state_fail() {
        let mut input = "- o]";
        assert!(parse_task_state(&mut input).is_err());
    }
}
