use chrono::NaiveTime;
use winnow::{combinator::separated, token::take_while, PResult, Parser};

use super::token::Token;

/// Parses a NaiveTime from a `hh:mm:ss` or `hh:mm` string.
pub fn parse_naive_time<'a>(input: &mut &'a str) -> PResult<Token> {
    let tokens: Vec<u32> =
        separated(2..=3, take_while(1.., '0'..='9').parse_to::<u32>(), ':').parse_next(input)?;

    let h = tokens[0];
    let m = tokens[1];
    let s = if tokens.len() == 3 { tokens[2] } else { 0 };

    Ok(Token::DueTime(NaiveTime::from_hms_opt(h, m, s).unwrap()))
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveTime, Timelike};

    use crate::parser::{parser_time::parse_naive_time, token::Token};

    #[test]
    fn test_parse_naive_time() {
        let now = chrono::Local::now().time();
        let (h, m, s) = (now.hour(), now.minute(), now.second());

        let input = format!("{h}:{m}:{s}");
        let expected = NaiveTime::from_hms_opt(h, m, s).unwrap();
        assert_eq!(
            parse_naive_time(&mut input.as_str()),
            Ok(Token::DueTime(expected))
        );

        let input = format!("{h}:{m}");
        let expected = NaiveTime::from_hms_opt(h, m, 0).unwrap();
        assert_eq!(
            parse_naive_time(&mut input.as_str()),
            Ok(Token::DueTime(expected))
        );
    }
}
