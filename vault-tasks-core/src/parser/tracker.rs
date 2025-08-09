use chrono::NaiveDateTime;
use tracing::error;
use winnow::{
    Parser, Result,
    ascii::{digit1, space0},
    combinator::{alt, delimited, opt, preceded, repeat, separated},
    token::{none_of, take_while},
};
mod parse_frequency;

use super::{parser_date, parser_time::parse_naive_time};
use crate::{
    TasksConfig,
    date::Date,
    tracker::{
        NewTracker, Tracker,
        tracker_category::{BoolEntry, NoteEntry, ScoreEntry, TrackerEntry},
    },
};
pub fn parse_tracker_definition(input: &mut &str, config: &TasksConfig) -> Result<NewTracker> {
    preceded(("Tracker:", space0), |input: &mut &str| {
        let name = take_while(0.., |c: char| c != '(')
            .map(|s: &str| s.trim().to_string())
            .parse_next(input)?;

        let start_date =
            delimited('(', |input: &mut &str| parse_date(config, input), ')').parse_next(input)?;
        Ok(NewTracker::new(name, start_date))
    })
    .parse_next(input)
}

fn parse_date(config: &TasksConfig, input: &mut &str) -> Result<Date, winnow::error::ContextError> {
    // Parse the date first
    let start_date = parser_date::parse_naive_date(input, config.core.use_american_format)?;
    // Try to parse optional time after the date
    let start_time_opt = opt(preceded(space0, parse_naive_time)).parse_next(input)?;

    // Combine date and time
    let start_date = match start_time_opt {
        Some(time) => Date::DayTime(NaiveDateTime::new(start_date, time)),
        None => Date::Day(start_date),
    };

    Ok(start_date)
}
pub fn parse_header(
    new_tracker: &NewTracker,
    filename: String,
    line_number: usize,
    input: &mut &str,
) -> Result<Tracker> {
    let frequency = preceded(
        '|',
        delimited(space0, parse_frequency::parse_frequency, space0),
    )
    .parse_next(input)?;
    let categories: Vec<String> = preceded(
        '|',
        separated(
            0..,
            repeat(1.., none_of('|'))
                .fold(String::new, |mut string, c| {
                    string.push(c);
                    string
                })
                .map(|cat| cat.trim().to_owned()),
            '|',
        ),
    )
    .parse_next(input)?;
    Ok(new_tracker.to_tracker(filename, line_number, frequency, categories))
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
    delimited('[', alt((' ', 'x')), ']')
        .parse_next(input)
        .map(|char| {
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

    let mut parsed_entries = vec![];
    for (n, entry) in entries.iter().enumerate() {
        let entry = entry.trim().to_string();

        // We're iterating over the entries we read from a line of the tracker
        // Either
        // - We already have at least one line from the categories
        // - It's the first line
        //
        // It's the n-th entry, is there a category for it?
        if let Some(cat) = tracker.categories.get(n) {
            // Is it the first entry we are parsing?
            parsed_entries.push(if let Some(first_entry) = cat.entries.first() {
                if entry.is_empty() {
                    // Empty entry => it's a blank entry
                    TrackerEntry::Blank
                } else {
                    // Else, entry's type must match the category's type, else it's a parsing error
                    match first_entry {
                        TrackerEntry::Score(_score_entry) => {
                            parse_score_entry(&mut entry.as_str())?
                        }
                        TrackerEntry::Bool(_bool_entry) => parse_bool_entry(&mut entry.as_str())?,
                        TrackerEntry::Note(_note_entry) => TrackerEntry::Note(NoteEntry {
                            value: entry.to_string().trim().to_owned(),
                        }),
                        TrackerEntry::Blank => TrackerEntry::Note(NoteEntry {
                            value: entry.to_string(), // The only way to get here is if the first entry was
                                                      // empty, but we know entry is not empty so that means
                                                      // the first entry was an empty Note entry
                        }),
                    }
                }
            } else {
                // No previous entries, we'll guess the type from the first entry
                if entry.is_empty() {
                    // Empty entry => it's a blank entry
                    TrackerEntry::Blank
                } else {
                    alt((
                        parse_score_entry,
                        parse_bool_entry,
                        (repeat(1.., none_of('|'))
                            .fold(String::new, |mut string, c| {
                                string.push(c);
                                string
                            })
                            .map(|s| {
                                TrackerEntry::Note(NoteEntry {
                                    value: s.trim().to_owned(),
                                })
                            })),
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
    use crate::{
        TasksConfig,
        date::Date,
        tracker::{
            NewTracker, Tracker,
            frequency::Frequency,
            tracker_category::{BoolEntry, NoteEntry, ScoreEntry, TrackerCategory, TrackerEntry},
        },
    };
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

    fn default_config() -> TasksConfig {
        TasksConfig {
            ..Default::default()
        }
    }

    #[test]
    fn test_parse_tracker_definition_basic() {
        let mut input = "Tracker: Reading Habit (2025/06/08)";
        let config = default_config();

        let result = parse_tracker_definition(&mut input, &config).unwrap();

        assert_eq!(result.name, "Reading Habit");
        assert_eq!(
            result.start_date,
            Date::Day(NaiveDate::from_ymd_opt(2025, 6, 8).unwrap())
        );
    }

    #[test]
    fn test_parse_tracker_definition_with_time() {
        let mut input = "Tracker: Daily Exercise (2025/06/08 14:30)";
        let config = default_config();

        let result = parse_tracker_definition(&mut input, &config).unwrap();

        assert_eq!(result.name, "Daily Exercise");
        assert_eq!(
            result.start_date,
            Date::DayTime(NaiveDateTime::new(
                NaiveDate::from_ymd_opt(2025, 6, 8).unwrap(),
                NaiveTime::from_hms_opt(14, 30, 0).unwrap()
            ))
        );
    }

    #[test]
    fn test_parse_tracker_definition_with_spaces() {
        let mut input = "Tracker:   My Daily Habits   (2025/06/08)";
        let config = default_config();

        let result = parse_tracker_definition(&mut input, &config).unwrap();

        assert_eq!(result.name, "My Daily Habits");
    }

    #[test]
    fn test_parse_tracker_definition_empty_name() {
        let mut input = "Tracker: (2025/06/08)";
        let config = default_config();

        let result = parse_tracker_definition(&mut input, &config).unwrap();

        assert_eq!(result.name, "");
    }

    #[test]
    fn test_parse_tracker_definition_invalid_format() {
        let mut input = "Invalid format";
        let config = default_config();

        let result = parse_tracker_definition(&mut input, &config);

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_date_day_only() {
        let mut input = "2025/06/08";
        let config = default_config();

        let result = parse_date(&config, &mut input).unwrap();

        assert_eq!(
            result,
            Date::Day(NaiveDate::from_ymd_opt(2025, 6, 8).unwrap())
        );
    }

    #[test]
    fn test_parse_date_with_time() {
        let mut input = "2025/06/08 09:15";
        let config = default_config();

        let result = parse_date(&config, &mut input).unwrap();

        assert_eq!(
            result,
            Date::DayTime(NaiveDateTime::new(
                NaiveDate::from_ymd_opt(2025, 6, 8).unwrap(),
                NaiveTime::from_hms_opt(9, 15, 0).unwrap()
            ))
        );
    }

    #[test]
    fn test_parse_date_with_seconds() {
        let mut input = "2025/06/08 09:15:30";
        let config = default_config();

        let result = parse_date(&config, &mut input).unwrap();

        assert_eq!(
            result,
            Date::DayTime(NaiveDateTime::new(
                NaiveDate::from_ymd_opt(2025, 6, 8).unwrap(),
                NaiveTime::from_hms_opt(9, 15, 30).unwrap()
            ))
        );
    }

    #[test]
    fn test_parse_header_basic() {
        let new_tracker = NewTracker::new(
            "Test Tracker".to_string(),
            Date::Day(NaiveDate::from_ymd_opt(2025, 6, 8).unwrap()),
        );
        let mut input = "| every day | blogs | books | news | notes |";

        let result = parse_header(&new_tracker, String::new(), 0, &mut input).unwrap();

        assert_eq!(result.name, "Test Tracker");
        assert_eq!(result.categories.len(), 4);
        assert_eq!(result.categories[0].name, "blogs");
        assert_eq!(result.categories[1].name, "books");
        assert_eq!(result.categories[2].name, "news");
        assert_eq!(result.categories[3].name, "notes");
    }

    #[test]
    fn test_parse_header_with_spaces() {
        let new_tracker = NewTracker::new(
            "Test Tracker".to_string(),
            Date::Day(NaiveDate::from_ymd_opt(2025, 6, 8).unwrap()),
        );
        let mut input = "|  every week  |  exercise  |  diet  |  sleep  |";

        let result = parse_header(&new_tracker, String::new(), 0, &mut input).unwrap();

        assert_eq!(result.categories.len(), 3);
        assert_eq!(result.categories[0].name, "exercise");
        assert_eq!(result.categories[1].name, "diet");
        assert_eq!(result.categories[2].name, "sleep");
    }

    #[test]
    fn test_parse_header_single_category() {
        let new_tracker = NewTracker::new(
            "Single Cat".to_string(),
            Date::Day(NaiveDate::from_ymd_opt(2025, 6, 8).unwrap()),
        );
        let mut input = "| every day | reading |";

        let result = parse_header(&new_tracker, String::new(), 0, &mut input).unwrap();

        assert_eq!(result.categories.len(), 1);
        assert_eq!(result.categories[0].name, "reading");
    }

    #[test]
    fn test_parse_separator_basic() {
        let mut input = "| ---------- | ----- | ----- | ---- |";

        let result = parse_separator(&mut input);

        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_separator_varied_lengths() {
        let mut input = "| --- | ---------- | - | ------ |";

        let result = parse_separator(&mut input);

        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_separator_with_spaces() {
        let mut input = "|  -----  |  ---  |  --------  |";

        let result = parse_separator(&mut input);

        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_score_entry_valid() {
        let mut input = "5";

        let result = parse_score_entry(&mut input).unwrap();

        match result {
            TrackerEntry::Score(score_entry) => assert_eq!(score_entry.score, 5),
            _ => panic!("Expected Score entry"),
        }
    }

    #[test]
    fn test_parse_score_entry_zero() {
        let mut input = "0";

        let result = parse_score_entry(&mut input).unwrap();

        match result {
            TrackerEntry::Score(score_entry) => assert_eq!(score_entry.score, 0),
            _ => panic!("Expected Score entry"),
        }
    }

    #[test]
    fn test_parse_score_entry_large_number() {
        let mut input = "999";

        let result = parse_score_entry(&mut input).unwrap();

        match result {
            TrackerEntry::Score(score_entry) => assert_eq!(score_entry.score, 999),
            _ => panic!("Expected Score entry"),
        }
    }

    #[test]
    fn test_parse_score_entry_invalid() {
        let mut input = "abc";

        let result = parse_score_entry(&mut input);

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_bool_entry_checked() {
        let mut input = "[x]";

        let result = parse_bool_entry(&mut input).unwrap();

        match result {
            TrackerEntry::Bool(bool_entry) => assert!(bool_entry.value),
            _ => panic!("Expected Bool entry"),
        }
    }

    #[test]
    fn test_parse_bool_entry_unchecked() {
        let mut input = "[ ]";

        let result = parse_bool_entry(&mut input).unwrap();

        match result {
            TrackerEntry::Bool(bool_entry) => assert!(!bool_entry.value),
            _ => panic!("Expected Bool entry"),
        }
    }

    #[test]
    fn test_parse_bool_entry_invalid_format() {
        let mut input = "[y]";

        let result = parse_bool_entry(&mut input);

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_entries_mixed_types() {
        // Create a tracker with existing categories to test type matching
        let tracker = Tracker {
            name: "Test Tracker".to_string(),
            start_date: Date::Day(NaiveDate::from_ymd_opt(2025, 6, 8).unwrap()),
            length: 1,
            frequency: Frequency::Days(1),
            categories: vec![
                TrackerCategory {
                    name: "bool_cat".to_string(),
                    entries: vec![TrackerEntry::Bool(BoolEntry { value: true })],
                },
                TrackerCategory {
                    name: "score_cat".to_string(),
                    entries: vec![TrackerEntry::Score(ScoreEntry { score: 1 })],
                },
                TrackerCategory {
                    name: "note_cat".to_string(),
                    entries: vec![TrackerEntry::Note(NoteEntry {
                        value: "test".to_string(),
                    })],
                },
            ],
            filename: String::new(),
            line_number: 0,
        };

        let mut input = "| 2025/06/09 | [x] | 5 | finished reading |";
        let config = default_config();

        let result = parse_entries(&tracker, &config, &mut input).unwrap();

        assert_eq!(
            result.0,
            Date::Day(NaiveDate::from_ymd_opt(2025, 6, 9).unwrap())
        );
        assert_eq!(result.1.len(), 3);

        match &result.1[0] {
            TrackerEntry::Bool(bool_entry) => assert!(bool_entry.value),
            _ => panic!("Expected Bool entry"),
        }

        match &result.1[1] {
            TrackerEntry::Score(score_entry) => assert_eq!(score_entry.score, 5),
            _ => panic!("Expected Score entry"),
        }

        match &result.1[2] {
            TrackerEntry::Note(note_entry) => assert_eq!(note_entry.value, "finished reading"),
            _ => panic!("Expected Note entry"),
        }
    }

    #[test]
    fn test_parse_entries_with_blank_entries() {
        let tracker = Tracker {
            name: "Test Tracker".to_string(),
            start_date: Date::Day(NaiveDate::from_ymd_opt(2025, 6, 8).unwrap()),
            length: 1,
            frequency: Frequency::Days(1),
            categories: vec![
                TrackerCategory {
                    name: "bool_cat".to_string(),
                    entries: vec![TrackerEntry::Bool(BoolEntry { value: true })],
                },
                TrackerCategory {
                    name: "score_cat".to_string(),
                    entries: vec![TrackerEntry::Score(ScoreEntry { score: 1 })],
                },
            ],
            filename: String::new(),
            line_number: 0,
        };

        let mut input = "| 2025/06/09 | [ ] |  |";
        let config = default_config();

        let result = parse_entries(&tracker, &config, &mut input).unwrap();

        assert_eq!(result.1.len(), 2);

        match &result.1[0] {
            TrackerEntry::Bool(bool_entry) => assert!(!bool_entry.value),
            _ => panic!("Expected Bool entry"),
        }

        match &result.1[1] {
            TrackerEntry::Blank => {} // Expected blank entry
            _ => panic!("Expected Blank entry"),
        }
    }

    #[test]
    fn test_parse_entries_first_time_type_inference() {
        // Tracker with no existing entries - should infer types
        let tracker = Tracker {
            filename: String::new(),
            line_number: 0,
            name: "New Tracker".to_string(),
            start_date: Date::Day(NaiveDate::from_ymd_opt(2025, 6, 8).unwrap()),
            length: 0,
            frequency: Frequency::Days(1),
            categories: vec![
                TrackerCategory {
                    name: "cat1".to_string(),
                    entries: vec![],
                },
                TrackerCategory {
                    name: "cat2".to_string(),
                    entries: vec![],
                },
                TrackerCategory {
                    name: "cat3".to_string(),
                    entries: vec![],
                },
            ],
        };

        let mut input = "| 2025/06/08 | [x] | 3 | some note |";
        let config = default_config();

        let result = parse_entries(&tracker, &config, &mut input).unwrap();

        assert_eq!(result.1.len(), 3);

        match &result.1[0] {
            TrackerEntry::Bool(bool_entry) => assert!(bool_entry.value),
            _ => panic!("Expected Bool entry"),
        }

        match &result.1[1] {
            TrackerEntry::Score(score_entry) => assert_eq!(score_entry.score, 3),
            _ => panic!("Expected Score entry"),
        }

        match &result.1[2] {
            TrackerEntry::Note(note_entry) => assert_eq!(note_entry.value, "some note"),
            _ => panic!("Expected Note entry"),
        }
    }

    #[test]
    fn test_parse_entries_with_datetime() {
        let tracker = Tracker {
            filename: String::new(),
            line_number: 0,
            name: "Test Tracker".to_string(),
            start_date: Date::Day(NaiveDate::from_ymd_opt(2025, 6, 8).unwrap()),
            length: 1,
            frequency: Frequency::Days(1),
            categories: vec![TrackerCategory {
                name: "activity".to_string(),
                entries: vec![TrackerEntry::Bool(BoolEntry { value: false })],
            }],
        };

        let mut input = "| 2025/06/09 14:30 | [x] |";
        let config = default_config();

        let result = parse_entries(&tracker, &config, &mut input).unwrap();

        assert_eq!(
            result.0,
            Date::DayTime(NaiveDateTime::new(
                NaiveDate::from_ymd_opt(2025, 6, 9).unwrap(),
                NaiveTime::from_hms_opt(14, 30, 0).unwrap()
            ))
        );
    }

    #[test]
    fn test_parse_entries_empty_first_entry_becomes_blank() {
        let tracker = Tracker {
            filename: String::new(),
            line_number: 0,
            name: "Test Tracker".to_string(),
            start_date: Date::Day(NaiveDate::from_ymd_opt(2025, 6, 8).unwrap()),
            length: 0,
            frequency: Frequency::Days(1),
            categories: vec![TrackerCategory {
                name: "cat1".to_string(),
                entries: vec![],
            }],
        };

        let mut input = "| 2025/06/08 |  |";
        let config = default_config();

        let result = parse_entries(&tracker, &config, &mut input).unwrap();

        assert_eq!(result.1.len(), 1);

        match &result.1[0] {
            TrackerEntry::Blank => {} // Expected blank entry
            _ => panic!("Expected Blank entry, got {:?}", result.1[0]),
        }
    }

    #[test]
    fn test_parse_entries_note_with_special_characters() {
        let tracker = Tracker {
            filename: String::new(),
            line_number: 0,
            name: "Test Tracker".to_string(),
            start_date: Date::Day(NaiveDate::from_ymd_opt(2025, 6, 8).unwrap()),
            length: 1,
            frequency: Frequency::Days(1),
            categories: vec![TrackerCategory {
                name: "notes".to_string(),
                entries: vec![TrackerEntry::Note(NoteEntry {
                    value: "test".to_string(),
                })],
            }],
        };

        let mut input = "| 2025/06/09 | Read 'The Great Gatsby' - 50% done! |";
        let config = default_config();

        let result = parse_entries(&tracker, &config, &mut input).unwrap();

        match &result.1[0] {
            TrackerEntry::Note(note_entry) => {
                assert_eq!(note_entry.value, "Read 'The Great Gatsby' - 50% done!");
            }
            _ => panic!("Expected Note entry"),
        }
    }

    #[test]
    fn test_parse_entries_score_type_mismatch_error() {
        let tracker = Tracker {
            filename: String::new(),
            line_number: 0,
            name: "Test Tracker".to_string(),
            start_date: Date::Day(NaiveDate::from_ymd_opt(2025, 6, 8).unwrap()),
            length: 1,
            frequency: Frequency::Days(1),
            categories: vec![TrackerCategory {
                name: "score_cat".to_string(),
                entries: vec![TrackerEntry::Score(ScoreEntry { score: 1 })],
            }],
        };

        let mut input = "| 2025/06/09 | [x] |"; // Bool format for score category
        let config = default_config();

        let result = parse_entries(&tracker, &config, &mut input);

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_entries_trimming_whitespace() {
        let tracker = Tracker {
            filename: String::new(),
            line_number: 0,
            name: "Test Tracker".to_string(),
            start_date: Date::Day(NaiveDate::from_ymd_opt(2025, 6, 8).unwrap()),
            length: 1,
            frequency: Frequency::Days(1),
            categories: vec![
                TrackerCategory {
                    name: "notes".to_string(),
                    entries: vec![TrackerEntry::Note(NoteEntry {
                        value: "test".to_string(),
                    })],
                },
                TrackerCategory {
                    name: "score".to_string(),
                    entries: vec![TrackerEntry::Score(ScoreEntry { score: 1 })],
                },
            ],
        };

        let mut input = "|  2025/06/09  |   trimmed note   |   42   |";
        let config = default_config();

        let result = parse_entries(&tracker, &config, &mut input).unwrap();

        match &result.1[0] {
            TrackerEntry::Note(note_entry) => {
                assert_eq!(note_entry.value, "trimmed note");
            }
            _ => panic!("Expected Note entry"),
        }

        match &result.1[1] {
            TrackerEntry::Score(score_entry) => assert_eq!(score_entry.score, 42),
            _ => panic!("Expected Score entry"),
        }
    }

    #[test]
    fn test_integration_full_tracker_parsing() {
        let config = default_config();

        // Parse tracker definition
        let mut def_input = "Tracker: Reading Habit (2025/06/08)";
        let new_tracker = parse_tracker_definition(&mut def_input, &config).unwrap();

        // Parse header
        let mut header_input = "| every day | blogs | books | news | notes |";
        let tracker = parse_header(&new_tracker, String::new(), 0, &mut header_input).unwrap();

        // Parse separator (just verify it works)
        let mut sep_input = "| ---------- | ----- | ----- | ---- | ------------------ |";
        parse_separator(&mut sep_input).unwrap();

        // Parse multiple entries
        let mut entry1_input = "| 2025/06/08 | [x] | [x] | 5 |  |";
        let (date1, entries1) = parse_entries(&tracker, &config, &mut entry1_input).unwrap();

        let mut entry2_input = "| 2025/06/09 | [x] | [ ] | 1 | finished this book |";
        let (date2, entries2) = parse_entries(&tracker, &config, &mut entry2_input).unwrap();

        // Verify results
        assert_eq!(tracker.name, "Reading Habit");
        assert_eq!(tracker.categories.len(), 4);

        assert_eq!(
            date1,
            Date::Day(NaiveDate::from_ymd_opt(2025, 6, 8).unwrap())
        );
        assert_eq!(entries1.len(), 4);

        assert_eq!(
            date2,
            Date::Day(NaiveDate::from_ymd_opt(2025, 6, 9).unwrap())
        );
        assert_eq!(entries2.len(), 4);

        // Verify specific entry values from second row
        match &entries2[3] {
            TrackerEntry::Note(note) => assert_eq!(note.value, "finished this book"),
            _ => panic!("Expected note entry"),
        }
    }

    #[test]
    fn test_error_cases() {
        let config = default_config();

        // Missing tracker prefix
        let mut input1 = "Reading Habit (2025/06/08)";
        assert!(parse_tracker_definition(&mut input1, &config).is_err());

        // Missing date parentheses
        let mut input2 = "Tracker: Reading Habit 2025/06/08";
        assert!(parse_tracker_definition(&mut input2, &config).is_err());

        // Invalid date format
        let mut input3 = "Tracker: Reading Habit (invalid-date)";
        assert!(parse_tracker_definition(&mut input3, &config).is_err());

        // Missing pipe separators in header
        let new_tracker = NewTracker::new(
            "Test".to_string(),
            Date::Day(NaiveDate::from_ymd_opt(2025, 6, 8).unwrap()),
        );
        let mut input4 = "every day blogs books";
        assert!(parse_header(&new_tracker, String::new(), 0, &mut input4).is_err());
    }
}
