use winnow::{combinator::preceded, token::take_while, Parser, Result};

use super::token::Token;

/// Parses a completion percentage of the form `"<integer>%"`.
pub fn parse_completion(input: &mut &str) -> Result<Token> {
    let res = preceded('c', take_while(1.., '0'..='9'))
        .parse_to()
        .parse_next(input)?;

    Ok(Token::CompletionPercentage(res))
}

#[cfg(test)]
mod tests {
    use crate::core::parser::task::{parse_completion::parse_completion, token::Token};

    #[test]
    fn test_parse_completion_success() {
        let mut with_tag = "c99";
        assert_eq!(
            parse_completion(&mut with_tag),
            Ok(Token::CompletionPercentage(99))
        );
    }
    #[test]
    fn test_parse_completion_fail() {
        let mut without_tag = "test";
        assert!(parse_completion(&mut without_tag).is_err());
    }
}
