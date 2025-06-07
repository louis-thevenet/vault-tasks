use chrono::NaiveTime;
use winnow::{Result, combinator::separated, prelude::*, token::take_while};

use super::token::Token;

/// Parses a `NaiveTime` from a `hh:mm:ss` or `hh:mm` string.
pub fn parse_naive_time(input: &mut &str) -> Result<Token> {
    separated(2..=3, take_while(1.., '0'..='9').parse_to::<u32>(), ':')
        .verify_map(|tokens: Vec<u32>| {
            let h = tokens[0];
            let m = tokens[1];
            let s = if tokens.len() == 3 { tokens[2] } else { 0 };
            if h < 24 && m < 60 && s < 60 {
                Some(Token::DueTime(NaiveTime::from_hms_opt(h, m, s).unwrap()))
            } else {
                None
            }
        })
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveTime, Timelike};

    use crate::core::parser::task::{parser_time::parse_naive_time, token::Token};

    #[test]
    fn test_parse_wrong_naive_time() {
        let input = "24:50".to_string();
        assert!(parse_naive_time(&mut input.as_str()).is_err());
        let input = "22:90".to_string();
        assert!(parse_naive_time(&mut input.as_str()).is_err());
        let input = "23:50:61".to_string();
        assert!(parse_naive_time(&mut input.as_str()).is_err());
    }
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
