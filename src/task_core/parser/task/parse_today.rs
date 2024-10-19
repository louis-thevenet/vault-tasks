use winnow::{
    combinator::{alt, preceded},
    PResult, Parser,
};

use super::token::Token;

/// Parses a `Token::TodayFlag` of the form of the form "@t", @tdy", "@tod" or "@today".
pub fn parse_today(input: &mut &str) -> PResult<Token> {
    preceded('@', alt(("today", "tod", "tdy", "t"))).parse_next(input)?;
    Ok(Token::TodayFlag)
}

#[cfg(test)]
mod tests {
    use crate::task_core::parser::task::{parse_today::parse_today, token::Token};

    #[test]
    fn test_parse_today_tag() {
        let mut with_today = "@t";
        assert_eq!(parse_today(&mut with_today), Ok(Token::TodayFlag));
        let mut with_today = "@today";
        assert_eq!(parse_today(&mut with_today), Ok(Token::TodayFlag));
        let mut with_today = "@tdy";
        assert_eq!(parse_today(&mut with_today), Ok(Token::TodayFlag));
    }
    #[test]
    fn test_parse_today_tag_fail() {
        let mut should_fail = "today";
        assert!(parse_today(&mut should_fail).is_err());
    }
}
