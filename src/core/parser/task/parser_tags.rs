use winnow::{combinator::preceded, token::take_while, PResult, Parser};

use super::token::Token;

/// Parses tags of the form "#tag".
pub fn parse_tag(input: &mut &str) -> PResult<Token> {
    let tag = preceded(
        '#',
        take_while(1.., ('_', '0'..='9', 'A'..='Z', 'a'..='z', '0'..='9')),
    )
    .parse_next(input)?;
    Ok(Token::Tag(tag.to_string()))
}

#[cfg(test)]
mod tests {
    use crate::core::parser::task::{parser_tags::parse_tag, token::Token};

    #[test]
    fn test_parse_tag_sucess() {
        let mut with_tag = "#test";
        assert_eq!(parse_tag(&mut with_tag), Ok(Token::Tag("test".to_string())));
    }
    #[test]
    fn test_parse_tag_symbols() {
        let mut with_tag = "#test_underscore123";
        assert_eq!(
            parse_tag(&mut with_tag),
            Ok(Token::Tag("test_underscore123".to_string()))
        );
    }
    #[test]
    fn test_parse_tag_fail() {
        let mut without_tag = "test";
        assert!(parse_tag(&mut without_tag).is_err());
    }
}
