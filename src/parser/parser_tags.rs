use winnow::{combinator::preceded, token::take_while, PResult, Parser};

use super::token::Token;

/// Parses tags of the form "#tag".
pub fn parse_tag<'a>(input: &mut &'a str) -> PResult<Token> {
    let tag =
        preceded('#', take_while(1.., ('0'..='9', 'A'..='Z', 'a'..='z'))).parse_next(input)?;
    Ok(Token::Tag(tag.to_string()))
}

#[cfg(test)]
mod tests {
    use crate::parser::{parser_tags::parse_tag, token::Token};

    #[test]
    fn test_parse_tag_sucess() {
        let mut with_tag = "#test";
        assert_eq!(parse_tag(&mut with_tag), Ok(Token::Tag("test".to_string())));
    }
    #[test]
    fn test_parse_tag_fail() {
        let mut without_tag = "test";
        assert!(parse_tag(&mut without_tag).is_err());
    }
}
