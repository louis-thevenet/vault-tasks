use std::any::Any;

use chrono::NaiveDateTime;
use tracing::{debug, error};
use winnow::{
    Parser, Result,
    ascii::{digit1, space0},
    combinator::{alt, delimited, opt, preceded, repeat, separated},
    token::{self, none_of, take_while},
};
mod parse_frequency;
use super::{parser_date, parser_time::parse_naive_time};
use crate::core::{
    TasksConfig,
    date::Date,
    tracker::{
        NewTracker, Tracker,
        tracker_category::{BoolEntry, NoteEntry, ScoreEntry, TrackerEntry},
    },
};
pub fn parse_tracker_definition(input: &mut &str, config: &TasksConfig) -> Result<NewTracker> {
    preceded(("Tracker:", space0), |input: &mut &str| {
        // Parse tracker name - everything up to the opening parenthesis
        let name = take_while(0.., |c: char| c != '(' && c != '\n')
            .map(|s: &str| s.trim().to_string())
            .parse_next(input)?;

        let start_date =
            delimited('(', |input: &mut &str| parse_date(config, input), ')').parse_next(input)?;

        Ok(NewTracker::new(name, start_date))
    })
    .parse_next(input)
}

fn parse_date(config: &TasksConfig, input: &mut &str) -> Result<Date, winnow::error::ContextError> {
    let start_date = {
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
        }
    }
    .parse_next(input)?;
    Ok(start_date)
}
pub fn parse_header(new_tracker: &NewTracker, input: &mut &str) -> Result<Tracker> {
    let frequency = preceded(
        '|',
        delimited(space0, parse_frequency::parse_frequency, space0),
    )
    .parse_next(input)?;
    let mut categories: Vec<String> = preceded(
        '|',
        separated(
            0..,
            repeat(1.., delimited(space0, none_of('|'), space0)).fold(
                String::new,
                |mut string, c| {
                    string.push(c);
                    string
                },
            ),
            '|',
        ),
    )
    .parse_next(input)?;
    Ok(new_tracker.to_tracker(frequency, categories))
}
pub fn parse_separator(input: &mut &str) -> Result<()> {
    '|'.parse_next(input)?;
    loop {
        delimited(space0, take_while(1.., '-'), space0).parse_next(input)?;
        '|'.parse_next(input)?;
        if input.is_empty() {
            break;
        }
    }

    Ok(())
}
fn parse_score_entry(input: &mut &str) -> Result<TrackerEntry> {
    digit1
        .parse_to()
        .map(|s: i32| TrackerEntry::Score(ScoreEntry { score: s }))
        .parse_next(input)
}
fn parse_bool_entry(input: &mut &str) -> Result<TrackerEntry> {
    alt((' ', 'x')).parse_next(input).map(|char| {
        if char == ' ' {
            TrackerEntry::Bool(BoolEntry { value: false })
        } else {
            TrackerEntry::Bool(BoolEntry { value: true })
        }
    })
}
pub fn parse_entries(
    tracker: &Tracker,
    config: &TasksConfig,
    input: &mut &str,
) -> Result<(Date, Vec<TrackerEntry>)> {
    let date: Date = preceded(
        '|',
        delimited(space0, |input: &mut &str| parse_date(config, input), space0),
    )
    .parse_next(input)?;

    // ensure date is consistent with tracker start date

    let entries: Vec<String> = preceded(
        '|',
        separated(
            0..,
            repeat(1.., none_of('|')).fold(String::new, |mut string, c| {
                string.push(c);
                string
            }),
            '|',
        ),
    )
    .parse_next(input)?;
    debug!("Parsed entries: {entries:?}");

    let mut parsed_entries = vec![];
    for (n, entry) in entries.iter().enumerate() {
        debug!("Parsed entry: {entry}");
        if let Some(cat) = tracker.categories.get(n) {
            parsed_entries.push(if let Some(first_entry) = cat.entries.first() {
                match first_entry {
                    TrackerEntry::Score(_score_entry) => parse_score_entry(&mut entry.as_str())?,
                    TrackerEntry::Bool(_bool_entry) => parse_bool_entry(&mut entry.as_str())?,
                    TrackerEntry::Note(_note_entry) => TrackerEntry::Note(NoteEntry {
                        value: entry.to_string(),
                    }),
                }
            } else {
                debug!("First entry, can't ensure type");
                if entry.is_empty() {
                    parse_bool_entry(&mut entry.as_str())?
                } else {
                    alt((
                        parse_score_entry,
                        (repeat(1.., none_of('|'))
                            .fold(String::new, |mut string, c| {
                                string.push(c);
                                string
                            })
                            .map(|s| TrackerEntry::Note(NoteEntry { value: s }))),
                    ))
                    .parse_next(&mut entry.as_str())?
                }
            });
        } else {
            error!("Tracker entries do not match categories: {tracker:?} {entries:?}");
        }
    }

    Ok((date, parsed_entries))
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
