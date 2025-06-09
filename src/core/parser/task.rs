mod parse_completion;
mod parse_today;
mod parser_priorities;
mod parser_state;
mod parser_tags;
mod token;

use chrono::NaiveDateTime;
use parse_completion::parse_completion;
use parse_today::parse_today;
use parser_priorities::parse_priority;
use parser_state::parse_task_state;
use parser_tags::parse_tag;
use token::Token;
use tracing::error;
use winnow::{
    Parser, Result,
    combinator::{alt, fail, repeat},
    token::any,
};

use crate::core::{TasksConfig, date::Date, task::Task};

use super::{
    parser_date::{self},
    parser_time,
};

/// Parses a `Token` from an input string.FileEntry
fn parse_token(input: &mut &str, config: &TasksConfig) -> Result<Token> {
    alt((
        // |input: &mut &str| (parse_naive_date(input, config.use_american_format)),
        |input: &mut &str| {
            let date = parser_date::parse_naive_date(input, config.use_american_format)?;
            Ok(Token::DueDate(date))
        },
        |input: &mut &str| {
            let date = parser_time::parse_naive_time(input)?;
            Ok(Token::DueTime(date))
        },
        parse_tag,
        |input: &mut &str| parse_task_state(input, &config.task_state_markers),
        parse_priority,
        parse_completion,
        parse_today,
        |input: &mut &str| {
            let res = repeat(0.., any)
                .fold(String::new, |mut string, c| {
                    string.push(c);
                    string
                })
                .parse_next(input)?;
            Ok(Token::Name(res))
        },
    ))
    .parse_next(input)
}

/// Parses a `Task` from an input string. Filename must be specified to be added to the task.
///
/// # Errors
///
/// Will return an error if the task can't be parsed.
#[allow(clippy::module_name_repetitions)]
pub fn parse_task(input: &mut &str, filename: String, config: &TasksConfig) -> Result<Task> {
    let task_state = match parse_task_state(input, &config.task_state_markers)? {
        Token::State(state) => Ok(state),
        _ => fail(input),
    }?;

    let mut token_parser = |input: &mut &str| parse_token(input, config);

    let tokens = input
        .split_ascii_whitespace()
        .map(|token| token_parser.parse(token));

    let mut task = Task {
        state: task_state,
        filename,
        ..Default::default()
    };

    // Placeholders for a date and a time
    let mut due_date_opt = None;
    let mut due_time_opt = None;
    let mut name_vec = vec![]; // collects words that aren't tokens from the input string

    for token_res in tokens {
        match token_res {
            Ok(Token::DueDate(date)) => due_date_opt = Some(date),
            Ok(Token::DueTime(time)) => due_time_opt = Some(time),
            Ok(Token::Name(name)) => name_vec.push(name),
            Ok(Token::Priority(p)) => task.priority = p,
            Ok(Token::State(state)) => task.state = state,
            Ok(Token::Tag(tag)) => {
                if let Some(ref mut tags) = task.tags {
                    tags.push(tag);
                } else {
                    task.tags = Some(vec![tag]);
                }
            }
            Ok(Token::TodayFlag) => task.is_today = true,
            Ok(Token::CompletionPercentage(c)) => task.completion = Some(c),
            Err(error) => error!("Error: {error:?}"),
        }
    }

    if !name_vec.is_empty() {
        task.name = name_vec.join(" ");
    }
    let due_date = if let Some(due_date) = due_date_opt {
        match due_time_opt {
            Some(time) => Some(Date::DayTime(NaiveDateTime::new(due_date, time))),
            None => Some(Date::Day(due_date)),
        }
    } else {
        None
    };

    task.due_date = due_date;
    Ok(task)
}
#[cfg(test)]
mod test {

    use chrono::{Datelike, Days, NaiveDate, NaiveDateTime, NaiveTime};

    use crate::core::{
        TasksConfig,
        date::Date,
        parser::task::parse_task,
        task::{State, Task},
    };
    #[test]
    fn test_parse_task_no_description() {
        let mut input = "- [x] 10/15 task_name #done";
        let config = TasksConfig {
            use_american_format: true,
            ..Default::default()
        };
        let res = parse_task(&mut input, String::new(), &config);
        assert!(res.is_ok());
        let res = res.unwrap();
        let year = chrono::Local::now().year();
        let expected = Task {
            name: "task_name".to_string(),
            description: None,
            tags: Some(vec!["done".to_string()]),
            due_date: Some(Date::Day(NaiveDate::from_ymd_opt(year, 10, 15).unwrap())),
            priority: 0,
            state: State::Done,
            line_number: Some(1),
            ..Default::default()
        };
        assert_eq!(res, expected);
    }

    #[test]
    fn test_parse_task_only_state() {
        let mut input = "- [ ]";
        let config = TasksConfig::default();
        let res = parse_task(&mut input, String::new(), &config);
        assert!(res.is_ok());
        let res = res.unwrap();
        let expected = Task {
            subtasks: vec![],
            name: String::new(),
            description: None,
            tags: None,
            due_date: None,
            priority: 0,
            state: State::ToDo,
            line_number: Some(1),
            filename: String::new(),
            is_today: false,
            completion: None,
        };
        assert_eq!(res, expected);
    }
    #[test]
    fn test_parse_task_with_due_date_words() {
        let mut input = "- [ ] today 15:30 task_name";
        let config = TasksConfig::default();
        let res = parse_task(&mut input, String::new(), &config);
        assert!(res.is_ok());
        let res = res.unwrap();
        let expected_date = chrono::Local::now().date_naive();
        let expected_time = NaiveTime::from_hms_opt(15, 30, 0).unwrap();
        let expected_due_date = Date::DayTime(NaiveDateTime::new(expected_date, expected_time));
        assert_eq!(res.due_date, Some(expected_due_date));
    }

    #[test]
    fn test_parse_task_with_weekday() {
        let mut input = "- [ ] monday 15:30 task_name";
        let config = TasksConfig::default();
        let res = parse_task(&mut input, String::new(), &config);
        assert!(res.is_ok());
        let res = res.unwrap();

        let now = chrono::Local::now();
        let expected_date = now
            .date_naive()
            .checked_add_days(Days::new(
                8 - u64::from(now.date_naive().weekday().number_from_monday()),
            ))
            .unwrap();
        let expected_time = NaiveTime::from_hms_opt(15, 30, 0).unwrap();
        let expected_due_date = Date::DayTime(NaiveDateTime::new(expected_date, expected_time));
        assert_eq!(res.due_date, Some(expected_due_date));
    }

    #[test]
    fn test_parse_task_with_weekday_this() {
        let mut input = "- [ ] this monday 15:30 task_name";
        let config = TasksConfig::default();
        let res = parse_task(&mut input, String::new(), &config);
        assert!(res.is_ok());
        let res = res.unwrap();
        let now = chrono::Local::now();
        let expected_date = now
            .date_naive()
            .checked_add_days(Days::new(
                8 - u64::from(now.date_naive().weekday().number_from_monday()),
            ))
            .unwrap();
        let expected_time = NaiveTime::from_hms_opt(15, 30, 0).unwrap();
        let expected_due_date = Date::DayTime(NaiveDateTime::new(expected_date, expected_time));
        assert_eq!(res.due_date, Some(expected_due_date));
    }

    #[test]
    fn test_parse_task_with_weekday_next() {
        let mut input = "- [ ] next monday 15:30 task_name";
        let config = TasksConfig::default();
        let res = parse_task(&mut input, String::new(), &config);
        assert!(res.is_ok());
        let res = res.unwrap();
        let now = chrono::Local::now();
        let expected_date = now
            .date_naive()
            .checked_add_days(Days::new(
                8 - u64::from(now.date_naive().weekday().number_from_monday()),
            ))
            .unwrap();
        let expected_time = NaiveTime::from_hms_opt(15, 30, 0).unwrap();
        let expected_due_date = Date::DayTime(NaiveDateTime::new(expected_date, expected_time));
        assert_eq!(res.due_date, Some(expected_due_date));
    }

    #[test]
    fn test_parse_task_without_due_date() {
        let mut input = "- [ ] task_name";
        let config = TasksConfig::default();
        let res = parse_task(&mut input, String::new(), &config);
        assert!(res.is_ok());
        let res = res.unwrap();
        let expected_due_date = None;
        assert_eq!(res.due_date, expected_due_date);
    }

    #[test]
    fn test_parse_task_with_invalid_state() {
        let mut input = "- [invalid] task_name";
        let config = TasksConfig::default();
        let res = parse_task(&mut input, String::new(), &config);
        assert!(res.is_err());
    }

    #[test]
    fn test_parse_task_without_state() {
        let mut input = "task_name";
        let config = TasksConfig::default();
        let res = parse_task(&mut input, String::new(), &config);
        assert!(res.is_err());
    }

    #[test]
    fn test_parse_task_with_invalid_priority() {
        let mut input = "- [ ] task_name p-9";
        let config = TasksConfig::default();
        let res = parse_task(&mut input, String::new(), &config);
        assert!(res.is_ok());
        let res = res.unwrap();
        assert_eq!(res.priority, 0);
    }

    #[test]
    fn test_parse_task_without_name() {
        let mut input = "- [ ]";
        let config = TasksConfig::default();
        let res = parse_task(&mut input, String::new(), &config);
        assert!(res.is_ok());
        let res = res.unwrap();
        assert_eq!(res.name, ""); // Default name is used when no name is provided
    }
    #[test]
    fn test_parse_task_with_today_flag() {
        let mut input = "- [ ] @t";
        let config = TasksConfig::default();
        let res = parse_task(&mut input, String::new(), &config);
        assert!(res.is_ok());
        let res = res.unwrap();
        assert!(res.is_today);
    }
}
