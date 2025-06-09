use chrono::NaiveDateTime;
use winnow::{
    Parser, Result,
    ascii::space0,
    combinator::{delimited, opt, preceded},
    token::take_while,
};
mod parse_frequency;
use super::{parser_date, parser_time::parse_naive_time};
use crate::core::{TasksConfig, date::Date, tracker::NewTracker};
pub fn parse_tracker_definition(input: &mut &str, config: &TasksConfig) -> Result<NewTracker> {
    preceded(("Tracker:", space0), |input: &mut &str| {
        // Parse tracker name - everything up to the opening parenthesis
        let name = take_while(0.., |c: char| c != '(' && c != '\n')
            .map(|s: &str| s.trim().to_string())
            .parse_next(input)?;

        // Parse optional date/time in parentheses
        let start_date = (delimited(
            '(',
            |input: &mut &str| {
                // Parse the date first
                let start_date = (|input: &mut &str| {
                    parser_date::parse_naive_date(input, config.use_american_format)
                })
                .parse_next(input)?;

                // Try to parse optional time after the date
                let start_time_opt = opt(preceded(space0, parse_naive_time)).parse_next(input)?;

                // Combine date and time
                let start_date = match start_time_opt {
                    Some(time) => Date::DayTime(NaiveDateTime::new(start_date, time)),
                    None => Date::Day(start_date),
                };

                Ok(start_date)
            },
            ')',
        ))
        .parse_next(input)?;

        Ok(NewTracker::new(name, start_date))
    })
    .parse_next(input)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::date::Date;
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

    #[test]
    fn test_parse_tracker_with_date() {
        let mut input = "Tracker: my tracker name (today 15:30)";
        let config = TasksConfig::default();
        let result = parse_tracker_definition(&mut input, &config);
        assert!(result.is_ok());
        let tracker = result.unwrap();
        assert_eq!(tracker.name, "my tracker name");
        let now = chrono::Local::now().naive_local();
        let expected_date = Date::DayTime(NaiveDateTime::new(
            now.date(),
            NaiveTime::from_hms_opt(15, 30, 0).unwrap(),
        ));
        assert_eq!(tracker.start_date, expected_date);
    }
    #[test]
    fn test_parse_tracker_with_date_only() {
        let mut input = "Tracker: date tracker (2024/12/25)";
        let config = TasksConfig {
            use_american_format: true,
            ..Default::default()
        };
        let result = parse_tracker_definition(&mut input, &config);
        assert!(result.is_ok());
        let tracker = result.unwrap();
        assert_eq!(tracker.name, "date tracker");
        let date = NaiveDate::from_ymd_opt(2024, 12, 25).unwrap();
        let expected_date = Date::Day(date);
        assert_eq!(tracker.start_date, expected_date);
    }
}
