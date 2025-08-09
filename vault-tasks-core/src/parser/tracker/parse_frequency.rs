use crate::tracker::frequency::Frequency;

use winnow::{
    Parser, Result,
    ascii::{digit1, space0, space1},
    combinator::{alt, preceded},
    error::ParserError,
};

fn parse_literal_generic<'a>(input: &mut &'a str) -> Result<&'a str> {
    let generics = (
        "minutes", "minute", "hours", "hour", "h", "days", "day", "d", "weeks", "week", "w",
        "months", "month", "m", "years", "year", "y",
    );
    alt(generics).parse_next(input)
}
fn string_to_frequency(freq: u64, s: &str) -> Result<Frequency> {
    match s {
        "minutes" | "minute" => Ok(Frequency::Minutes(freq)),
        "hours" | "hour" | "h" => Ok(Frequency::Hours(freq)),
        "days" | "day" | "d" => Ok(Frequency::Days(freq)),
        "weeks" | "week" | "w" => Ok(Frequency::Weeks(freq)),
        "months" | "month" | "m" => Ok(Frequency::Months(freq)),
        "years" | "year" | "y" => Ok(Frequency::Years(freq)),
        _ => Err(ParserError::from_input(&s)),
    }
}
fn parse_every_digit(input: &mut &str) -> Result<Frequency> {
    let number: u64 = preceded((alt(("every", "Every")), space1), digit1)
        .parse_to()
        .parse_next(input)?;
    let duration = preceded(space0, parse_literal_generic).parse_next(input)?; // allow space0 for input like 2d 2h etc
    string_to_frequency(number, duration)
}
fn parse_every(input: &mut &str) -> Result<Frequency> {
    let duration =
        preceded((alt(("every", "Every")), space1), parse_literal_generic).parse_next(input)?;
    string_to_frequency(1, duration)
}
fn parse_adverb(input: &mut &str) -> Result<Frequency> {
    let words = ("hourly", "daily", "weekly", "monthly", "yearly");
    let word = alt(words).parse_next(input)?;
    match word {
        "hourly" => Ok(Frequency::Hours(1)),

        "daily" => Ok(Frequency::Days(1)),
        "weekly" => Ok(Frequency::Weeks(1)),
        "monthly" => Ok(Frequency::Months(1)),
        "yearly" => Ok(Frequency::Years(1)),
        _ => Err(ParserError::from_input(input)),
    }
}
pub fn parse_frequency(input: &mut &str) -> Result<Frequency> {
    alt((parse_every_digit, parse_every, parse_adverb)).parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    use winnow::Result;
    // Test helper function to parse and unwrap results
    fn parse_freq(input: &str) -> Result<Frequency> {
        let mut input_str = input;
        parse_frequency(&mut input_str)
    }
    #[test]
    fn test_every_with_numbers() {
        assert_eq!(parse_freq("every 5minutes").unwrap(), Frequency::Minutes(5));
        assert_eq!(parse_freq("Every 2hours").unwrap(), Frequency::Hours(2));
        assert_eq!(parse_freq("every 3days").unwrap(), Frequency::Days(3));
        assert_eq!(parse_freq("every 4weeks").unwrap(), Frequency::Weeks(4));
        assert_eq!(parse_freq("every 6months").unwrap(), Frequency::Months(6));
        assert_eq!(parse_freq("every 2years").unwrap(), Frequency::Years(2));
    }

    #[test]
    fn test_every_without_numbers() {
        assert_eq!(parse_freq("every minute").unwrap(), Frequency::Minutes(1));
        assert_eq!(parse_freq("Every hour").unwrap(), Frequency::Hours(1));
        assert_eq!(parse_freq("every day").unwrap(), Frequency::Days(1));
        assert_eq!(parse_freq("every week").unwrap(), Frequency::Weeks(1));
        assert_eq!(parse_freq("every month").unwrap(), Frequency::Months(1));
        assert_eq!(parse_freq("every year").unwrap(), Frequency::Years(1));
    }

    #[test]
    fn test_short_forms() {
        assert_eq!(parse_freq("every 24h").unwrap(), Frequency::Hours(24));
        assert_eq!(parse_freq("every 7d").unwrap(), Frequency::Days(7));
        assert_eq!(parse_freq("every 2w").unwrap(), Frequency::Weeks(2));
        assert_eq!(parse_freq("every 12m").unwrap(), Frequency::Months(12));
        assert_eq!(parse_freq("every 1y").unwrap(), Frequency::Years(1));
    }

    #[test]
    fn test_singular_and_plural() {
        assert_eq!(parse_freq("every 1minute").unwrap(), Frequency::Minutes(1));
        assert_eq!(parse_freq("every 5minutes").unwrap(), Frequency::Minutes(5));
        assert_eq!(parse_freq("every 1hour").unwrap(), Frequency::Hours(1));
        assert_eq!(parse_freq("every 3hours").unwrap(), Frequency::Hours(3));
    }

    #[test]
    fn test_case_sensitivity() {
        assert_eq!(parse_freq("Every 5minutes").unwrap(), Frequency::Minutes(5));
        assert_eq!(parse_freq("every 2hours").unwrap(), Frequency::Hours(2));
    }

    #[test]
    fn test_large_numbers() {
        assert_eq!(
            parse_freq("every 999minutes").unwrap(),
            Frequency::Minutes(999)
        );
        assert_eq!(parse_freq("every 365days").unwrap(), Frequency::Days(365));
    }

    #[test]
    fn test_invalid_inputs() {
        // Missing "every"
        assert!(parse_freq("5minutes").is_err());
        assert!(parse_freq("minutes").is_err());

        // No time unit
        assert!(parse_freq("every 5").is_err());
        assert!(parse_freq("Every").is_err());

        // Empty input
        assert!(parse_freq("").is_err());
    }

    #[test]
    fn test_string_to_frequency_helper() {
        assert_eq!(string_to_frequency(5, "minutes"), Ok(Frequency::Minutes(5)));
        assert_eq!(string_to_frequency(1, "hour"), Ok(Frequency::Hours(1)));
        assert_eq!(string_to_frequency(3, "d"), Ok(Frequency::Days(3)));
        assert_eq!(string_to_frequency(2, "w"), Ok(Frequency::Weeks(2)));
        assert_eq!(string_to_frequency(6, "m"), Ok(Frequency::Months(6)));
        assert_eq!(string_to_frequency(1, "y"), Ok(Frequency::Years(1)));
    }
    #[test]
    fn test_adverbs() {
        assert_eq!(parse_freq("hourly").unwrap(), Frequency::Hours(1));
        assert_eq!(parse_freq("daily").unwrap(), Frequency::Days(1));
        assert_eq!(parse_freq("weekly").unwrap(), Frequency::Weeks(1));
        assert_eq!(parse_freq("monthly").unwrap(), Frequency::Months(1));
        assert_eq!(parse_freq("yearly").unwrap(), Frequency::Years(1));
    }

    #[test]
    fn test_adverbs_case_sensitivity() {
        // Test that adverbs are case-sensitive (they should fail with capitals)
        assert!(parse_freq("Hourly").is_err());
        assert!(parse_freq("Daily").is_err());
        assert!(parse_freq("Weekly").is_err());
        assert!(parse_freq("Monthly").is_err());
        assert!(parse_freq("Yearly").is_err());
        assert!(parse_freq("HOURLY").is_err());
    }

    #[test]
    fn test_invalid_adverbs() {
        // Test invalid adverb-like words
        assert!(parse_freq("secondly").is_err());
        assert!(parse_freq("minutely").is_err());
        assert!(parse_freq("biweekly").is_err());
        assert!(parse_freq("quarterly").is_err());
        assert!(parse_freq("annually").is_err()); // should use "yearly" instead
    }

    #[test]
    fn test_mixed_formats() {
        // Test that all three parsing functions work together
        assert_eq!(parse_freq("every 2hours").unwrap(), Frequency::Hours(2));
        assert_eq!(parse_freq("every hour").unwrap(), Frequency::Hours(1));
        assert_eq!(parse_freq("hourly").unwrap(), Frequency::Hours(1));

        assert_eq!(parse_freq("every 7days").unwrap(), Frequency::Days(7));
        assert_eq!(parse_freq("every day").unwrap(), Frequency::Days(1));
        assert_eq!(parse_freq("daily").unwrap(), Frequency::Days(1));
    }
}
