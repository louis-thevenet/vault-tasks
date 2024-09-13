use winnow::{combinator::preceded, token::take_while, PResult, Parser};

use super::token::Token;

/// Parses a priority value of the form `"p<integer>"`.
pub fn parse_priority(input: &mut &str) -> PResult<Token> {
    let res = preceded('p', take_while(1.., '0'..='9'))
        .parse_to()
        .parse_next(input)?;

    Ok(Token::Priority(res))
}

#[cfg(test)]
mod tests {
    use crate::task_core::parser::task::{parser_priorities::parse_priority, token::Token};

    #[test]
    fn test_parse_priority_sucess() {
        let mut with_tag = "p5";
        assert_eq!(parse_priority(&mut with_tag), Ok(Token::Priority(5)));
    }
    #[test]
    fn test_parse_priority_fail() {
        let mut without_tag = "test";
        assert!(parse_priority(&mut without_tag).is_err());
    }
}
