use chrono::{Datelike, Days, Months, NaiveDate};

use tracing::error;
use winnow::{
    Parser, Result,
    ascii::digit1,
    combinator::{alt, separated},
    error::ParserError,
    token::take_while,
};

/// Parses a literal day name (or its abbreviation) from an input string.
fn parse_literal_day<'a>(input: &mut &'a str) -> Result<&'a str> {
    let days = (
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
        "friday",
        "saturday",
        "sunday",
        "mon",
        "tue",
        "wed",
        "thu",
        "fri",
        "sat",
        "sun",
    );
    alt(days).parse_next(input)
}

/// Adds `n` days to today's date and returns it as a `NaiveDate`.
fn calculate_in_n_days(n: u32) -> NaiveDate {
    let now = chrono::Local::now();
    now.checked_add_days(Days::new(u64::from(n)))
        .unwrap()
        .date_naive()
}

/// Parses a `NaiveDate` from a literal day name
/// Day names can be abbreviated.
/// If successful, returns a `NaiveDate` representing the next occurrence of that day in the future (not including today).
fn parse_naive_date_from_literal_day(input: &mut &str) -> Result<NaiveDate> {
    let output = parse_literal_day.parse_next(input)?;
    let day: u32 = match &output[0..3] {
        "mon" => 1,
        "tue" => 2,
        "wed" => 3,
        "thu" => 4,
        "fri" => 5,
        "sat" => 6,
        "sun" => 7,
        _ => {
            error!("Unknown day name was parsed: {}", &output[0..3]);
            1
        }
    };
    let now = chrono::Local::now();
    let day_name_equals_today = now.weekday().number_from_monday() == day;

    let res = calculate_in_n_days(
        (if day_name_equals_today { 7 } else { 0 })
            + ((7 - now.weekday().num_days_from_sunday()) + day) % 7,
    );

    Ok(res)
}

/// Parses `("day", "week", "month", "year", "weekend", "we")` as a string from an input string.
fn parse_literal_generic<'a>(input: &mut &'a str) -> Result<&'a str> {
    let generics = (
        "days", "day", "d", "weeks", "week", "w", "months", "month", "m", "years", "year", "y",
    );
    alt(generics).parse_next(input)
}

/// Parses a `NaiveDate` from an integer + a generic duration in `("day", "week", "month", "year", "weekend", "we")`
/// If successful, returns a `NaiveDate` representing the start of the next generic duration found. "Next week" -> "Next Monday"
fn parse_naive_date_from_generic_name(input: &mut &str) -> Result<NaiveDate> {
    let number: u64 = digit1.parse_to().parse_next(input)?;
    let duration = parse_literal_generic.parse_next(input)?;

    let now = chrono::Local::now();
    let today_date = now.date_naive();
    match duration {
        "d" | "day" | "days" => Ok(today_date.checked_add_days(Days::new(number)).unwrap()),
        "w" | "week" | "weeks" => Ok(today_date
            .checked_add_days(Days::new(
                7 * (number - 1) + 8 - u64::from(now.weekday().number_from_monday()),
            ))
            .unwrap()),
        "m" | "month" | "months" => Ok(today_date
            .checked_add_months(Months::new(number.try_into().unwrap()))
            .unwrap()
            .checked_sub_days(Days::new(u64::from(today_date.day())))
            .unwrap()),
        "y" | "year" | "years" => Ok(today_date
            .checked_add_months(Months::new((12 * number).try_into().unwrap()))
            .unwrap()
            .with_month(1)
            .unwrap()
            .with_day(1)
            .unwrap()),
        _ => Ok(today_date),
    }
}

/// Parses `("tmr", "tomorrow", "today", "tdy", "tod")` as a string from an input string.
fn parse_adverb<'a>(input: &mut &'a str) -> Result<&'a str> {
    alt(("tmr", "tomorrow", "today", "tdy", "tod")).parse_next(input)
}

/// Parses a `NaiveDate` from an adverb in  `("tmr", "tomorrow", "today", "tdy", "tod")`
/// If successful, returns a `NaiveDate` representing today's or tomorrow's date
fn parse_naive_date_from_adverb(input: &mut &str) -> Result<NaiveDate> {
    let output = parse_adverb.parse_next(input)?;
    let now = chrono::Local::now();
    match output {
        #[allow(clippy::match_same_arms)]
        "tdy" | "tod" | "today" => Ok(now.date_naive()),
        "tmr" | "tomorrow" => Ok(now.date_naive().checked_add_days(Days::new(1)).unwrap()),
        _ => Err(ParserError::from_input(input)),
    }
}

/// Parses a `NaiveDate` from a `yyyy/mm/dd` string or `yyyy-mm-dd`.
/// Can change convention with the `american_format` flag.
fn parse_naive_date_from_numeric_format(
    input: &mut &str,
    american_format: bool,
) -> Result<NaiveDate> {
    let mut tokens: Vec<u32> = separated(
        2..=3,
        take_while(1.., '0'..='9').parse_to::<u32>(),
        alt(('/', '-')),
    )
    .parse_next(input)?;

    if !american_format {
        tokens.reverse();
    }
    if tokens.len() == 2 {
        tokens.insert(0, chrono::Local::now().year_ce().1);
    } else if tokens[0] < 100 {
        tokens[0] += 2000; // proleptic Gregorian year modulo 100
    }
    #[allow(clippy::cast_possible_wrap)]
    NaiveDate::from_ymd_opt(tokens[0] as i32, tokens[1], tokens[2])
        .map_or_else(|| Err(ParserError::from_input(input)), Ok)
}

/// Parses a `NaiveDate` from the following cases:
/// - "yyyy/mm/dd" (see `american_format` flag)
/// - "next <day name>", "next <day|week|month|year>"
/// - "<day name>"
/// - "tomorrow", "today"
///
/// Supports abbreviations
pub fn parse_naive_date(input: &mut &str, american_format: bool) -> Result<NaiveDate> {
    alt((
        (|input: &mut &str| parse_naive_date_from_numeric_format(input, american_format)),
        parse_naive_date_from_literal_day,
        parse_naive_date_from_adverb,
        parse_naive_date_from_generic_name,
    ))
    .parse_next(input)
}

/// For each functions that returns a `NaiveDate`, the complete parser `parse_due_date` is also tested to return the same result.
#[cfg(test)]
mod tests {
    use chrono::Datelike;

    use crate::core::parser::parser_date::*;

    #[test]
    fn test_parse_literal_day() {
        // Test with abbreviated day names
        let mut input = "mon";
        assert_eq!(parse_literal_day(&mut input), Ok("mon"));

        let mut input = "tue";
        assert_eq!(parse_literal_day(&mut input), Ok("tue"));

        let mut input = "Invalid day";
        assert!(parse_literal_day(&mut input).is_err());

        // Test with non-abbreviated day names
        let mut input = "monday";
        assert_eq!(parse_literal_day(&mut input), Ok("monday"));

        let mut input = "sunday";
        assert_eq!(parse_literal_day(&mut input), Ok("sunday"));
    }

    #[test]
    fn test_parse_naive_date_from_literal_day_opt_next() {
        // Test with today's day
        let now = chrono::Local::now();
        let days = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];
        let input = days[now.weekday().num_days_from_monday() as usize].to_string();
        let mut input = input.as_str();
        let mut copy = input;
        assert_eq!(
            parse_naive_date_from_literal_day(&mut input),
            Ok(calculate_in_n_days(7))
        );

        assert_eq!(
            parse_naive_date(&mut copy, true),
            Ok(calculate_in_n_days(7))
        );

        // Test with tomorrow's day
        let input = days[(1 + now.weekday().num_days_from_monday() as usize) % 7].to_string();
        let mut input = input.as_str();
        let mut copy = input;
        let expected = calculate_in_n_days(1);
        assert_eq!(parse_naive_date_from_literal_day(&mut input), Ok(expected));
        assert_eq!(parse_naive_date(&mut copy, true), Ok(expected));

        // Test with today's day without "next"

        let input = days[now.weekday().num_days_from_monday() as usize].to_string();
        let mut input = input.as_str();
        let mut copy = input;
        assert_eq!(
            parse_naive_date_from_literal_day(&mut input),
            Ok(calculate_in_n_days(7))
        );
        assert_eq!(
            parse_naive_date(&mut copy, true),
            Ok(calculate_in_n_days(7))
        );
    }

    #[test]
    fn test_parse_literal_generic() {
        // Test with different generic names
        let mut input = "day";
        assert_eq!(parse_literal_generic(&mut input), Ok("day"));

        let mut input = "week";
        assert_eq!(parse_literal_generic(&mut input), Ok("week"));
    }

    #[test]
    fn test_parse_naive_date_from_generic_name() {
        // Test with different generic names
        let mut input = "2day";
        let mut copy = input;
        assert_eq!(
            parse_naive_date_from_generic_name(&mut input),
            Ok(calculate_in_n_days(2))
        );
        assert_eq!(
            parse_naive_date(&mut copy, true),
            Ok(calculate_in_n_days(2))
        );

        let mut input = "4week";
        let mut copy = input;
        let now = chrono::Local::now();
        let expected = now
            .date_naive()
            .checked_add_days(Days::new(
                3 * 7 + 8 - u64::from(now.date_naive().weekday().number_from_monday()),
            ))
            .unwrap();
        assert_eq!(parse_naive_date_from_generic_name(&mut input), Ok(expected));
        assert_eq!(parse_naive_date(&mut copy, true), Ok(expected));
    }

    #[test]
    fn test_parse_adverb() {
        // Test with different adverbs
        let mut input = "tmr";
        assert_eq!(parse_adverb(&mut input), Ok("tmr"));

        let mut input = "today";
        assert_eq!(parse_adverb(&mut input), Ok("today"));
    }

    #[test]
    fn test_parse_naive_date_from_adverb() {
        // Test with different adverbs
        let mut input = "tdy";
        let mut copy = input;
        let now = chrono::Local::now();
        assert_eq!(
            parse_naive_date_from_adverb(&mut input),
            Ok(now.date_naive())
        );
        assert_eq!(parse_naive_date(&mut copy, true), Ok(now.date_naive()));

        let mut input = "tmr";
        let mut copy = input;
        let expected = now.date_naive().checked_add_days(Days::new(1)).unwrap();
        assert_eq!(parse_naive_date_from_adverb(&mut input), Ok(expected));
        assert_eq!(parse_naive_date(&mut copy, true), Ok(expected));
    }
    #[test]
    fn test_parse_naive_date_from_numeric_date() {
        let now = chrono::Local::now();
        let (y, m, d) = (now.year(), now.month(), now.day());

        let yyyy_mm_dd = format!("{y}/{m}/{d}");
        assert_eq!(
            parse_naive_date_from_numeric_format(&mut yyyy_mm_dd.as_str(), true),
            Ok(now.date_naive())
        );
        assert_eq!(
            parse_naive_date(&mut yyyy_mm_dd.as_str(), true),
            Ok(now.date_naive())
        );

        let dd_mm_yyyy = format!("{d}/{m}/{y}");
        assert_eq!(
            parse_naive_date_from_numeric_format(&mut dd_mm_yyyy.as_str(), false),
            Ok(now.date_naive())
        );
        assert_eq!(
            parse_naive_date(&mut dd_mm_yyyy.as_str(), false),
            Ok(now.date_naive())
        );
        let dd_mm = format!("{d}/{m}");
        assert_eq!(
            parse_naive_date_from_numeric_format(&mut dd_mm.as_str(), false),
            Ok(now.date_naive())
        );
        assert_eq!(
            parse_naive_date_from_numeric_format(&mut dd_mm.as_str(), false),
            Ok(now.date_naive())
        );

        let mm_incomplete = format!("{m}");
        assert!(parse_naive_date_from_numeric_format(&mut mm_incomplete.as_str(), false).is_err());
        assert!(parse_naive_date(&mut mm_incomplete.as_str(), false).is_err());
    }

    #[test]
    fn test_invalid_numeric_date() {
        let yyyy_mm_dd = "2024/63/17".to_string();
        assert!(parse_naive_date_from_numeric_format(&mut yyyy_mm_dd.as_str(), true).is_err());
    }
}
