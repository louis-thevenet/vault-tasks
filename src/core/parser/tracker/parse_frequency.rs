use crate::core::tracker::frequency::Frequency;

use color_eyre::eyre::bail;
use winnow::{
    Parser, Result,
    ascii::{digit1, space0, space1},
    combinator::{alt, fail, preceded, repeat},
    token::any,
};

fn parse_literal_generic<'a>(input: &mut &'a str) -> Result<&'a str> {
    let generics = (
        "minutes", "minute", "hours", "hour", "h", "days", "day", "d", "weeks", "week", "w",
        "months", "month", "m", "years", "year", "y",
    );
    alt(generics).parse_next(input)
}
fn string_to_frequency(freq: u64, s: &str) -> Frequency {
    match s {
        "minutes" | "minute" => Frequency::EveryXMinutes(freq),
        "hours" | "hour" | "h" => Frequency::EveryXHours(freq),
        "days" | "day" | "d" => Frequency::EveryXDays(freq),
        "weeks" | "week" | "w" => Frequency::EveryXWeeks(freq),
        "months" | "month" | "m" => Frequency::EveryXMonths(freq),
        _ => Frequency::EveryXYears(freq),
    }
}
fn parse_every_digit(input: &mut &str) -> Result<Frequency> {
    let number: u64 = preceded((alt(("every", "Every")), space1), digit1)
        .parse_to()
        .parse_next(input)?;
    let duration = preceded(space0, parse_literal_generic).parse_next(input)?; // allow space0 for input like 2d 2h etc
    Ok(string_to_frequency(number, duration))
}
fn parse_every(input: &mut &str) -> Result<Frequency> {
    let duration =
        preceded((alt(("every", "Every")), space1), parse_literal_generic).parse_next(input)?;
    Ok(string_to_frequency(1, duration))
}
fn parse_frequency(input: &mut &str) -> Result<Frequency> {
    alt((parse_every_digit, parse_every)).parse_next(input)
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
        assert_eq!(
            parse_freq("every 5minutes").unwrap(),
            Frequency::EveryXMinutes(5)
        );
        assert_eq!(
            parse_freq("Every 2hours").unwrap(),
            Frequency::EveryXHours(2)
        );
        assert_eq!(parse_freq("every 3days").unwrap(), Frequency::EveryXDays(3));
        assert_eq!(
            parse_freq("every 4weeks").unwrap(),
            Frequency::EveryXWeeks(4)
        );
        assert_eq!(
            parse_freq("every 6months").unwrap(),
            Frequency::EveryXMonths(6)
        );
        assert_eq!(
            parse_freq("every 2years").unwrap(),
            Frequency::EveryXYears(2)
        );
    }

    #[test]
    fn test_every_without_numbers() {
        assert_eq!(
            parse_freq("every minute").unwrap(),
            Frequency::EveryXMinutes(1)
        );
        assert_eq!(parse_freq("Every hour").unwrap(), Frequency::EveryXHours(1));
        assert_eq!(parse_freq("every day").unwrap(), Frequency::EveryXDays(1));
        assert_eq!(parse_freq("every week").unwrap(), Frequency::EveryXWeeks(1));
        assert_eq!(
            parse_freq("every month").unwrap(),
            Frequency::EveryXMonths(1)
        );
        assert_eq!(parse_freq("every year").unwrap(), Frequency::EveryXYears(1));
    }

    #[test]
    fn test_short_forms() {
        assert_eq!(parse_freq("every 24h").unwrap(), Frequency::EveryXHours(24));
        assert_eq!(parse_freq("every 7d").unwrap(), Frequency::EveryXDays(7));
        assert_eq!(parse_freq("every 2w").unwrap(), Frequency::EveryXWeeks(2));
        assert_eq!(
            parse_freq("every 12m").unwrap(),
            Frequency::EveryXMonths(12)
        );
        assert_eq!(parse_freq("every 1y").unwrap(), Frequency::EveryXYears(1));
    }

    #[test]
    fn test_singular_and_plural() {
        assert_eq!(
            parse_freq("every 1minute").unwrap(),
            Frequency::EveryXMinutes(1)
        );
        assert_eq!(
            parse_freq("every 5minutes").unwrap(),
            Frequency::EveryXMinutes(5)
        );
        assert_eq!(
            parse_freq("every 1hour").unwrap(),
            Frequency::EveryXHours(1)
        );
        assert_eq!(
            parse_freq("every 3hours").unwrap(),
            Frequency::EveryXHours(3)
        );
    }

    #[test]
    fn test_case_sensitivity() {
        assert_eq!(
            parse_freq("Every 5minutes").unwrap(),
            Frequency::EveryXMinutes(5)
        );
        assert_eq!(
            parse_freq("every 2hours").unwrap(),
            Frequency::EveryXHours(2)
        );
    }

    #[test]
    fn test_large_numbers() {
        assert_eq!(
            parse_freq("every 999minutes").unwrap(),
            Frequency::EveryXMinutes(999)
        );
        assert_eq!(
            parse_freq("every 365days").unwrap(),
            Frequency::EveryXDays(365)
        );
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
        assert_eq!(
            string_to_frequency(5, "minutes"),
            Frequency::EveryXMinutes(5)
        );
        assert_eq!(string_to_frequency(1, "hour"), Frequency::EveryXHours(1));
        assert_eq!(string_to_frequency(3, "d"), Frequency::EveryXDays(3));
        assert_eq!(string_to_frequency(2, "w"), Frequency::EveryXWeeks(2));
        assert_eq!(string_to_frequency(6, "m"), Frequency::EveryXMonths(6));
        assert_eq!(string_to_frequency(1, "y"), Frequency::EveryXYears(1));

        // Unknown unit defaults to years
        assert_eq!(
            string_to_frequency(10, "unknown"),
            Frequency::EveryXYears(10)
        );
    }
}
