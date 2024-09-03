mod parser_due_date;
mod parser_priorities;
mod parser_state;
mod parser_tags;
mod parser_time;
mod token;

use chrono::NaiveDateTime;
use log::error;
use parser_due_date::parse_naive_date;
use parser_priorities::parse_priority;
use parser_state::parse_task_state;
use parser_tags::parse_tag;
use parser_time::parse_naive_time;
use token::Token;
use winnow::{
    combinator::{alt, fail, repeat},
    token::any,
    PResult, Parser,
};

use crate::{
    config::Config,
    task::{DueDate, Task},
};

/// Parses a `Token` from an input string.
fn parse_token(input: &mut &str, config: &Config) -> PResult<Token> {
    alt((
        |input: &mut &str| parse_naive_date(input, config.use_american_format.unwrap_or(true)),
        parse_naive_time,
        parse_tag,
        parse_task_state,
        parse_priority,
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

/// Parses a `Task` from an input string. An optional description can be added.
pub fn parse_task(input: &mut &str, config: &Config) -> PResult<Task> {
    // `split_whitespace()` will break the "- [ ]" pattern
    let task_state = match parse_task_state(input)? {
        Token::State(state) => Ok(state),
        _ => fail(input),
    }?;

    let mut token_parser = |input: &mut &str| parse_token(input, config);

    let tokens = input.split_ascii_whitespace().map(|token| {
        token_parser
            .parse(token)
            .map_err(|e| anyhow::format_err!("{e}"))
    });

    let mut task = Task {
        state: task_state,
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
                    tags.push(tag)
                } else {
                    task.tags = Some(vec![tag])
                }
            }
            Err(error) => error!("Error: {error}"),
        }
    }

    if !name_vec.is_empty() {
        task.name = name_vec.join(" ");
    }

    let due_naive_date = due_date_opt.unwrap_or(chrono::Local::now().date_naive());
    let due_date = match due_time_opt {
        Some(time) => DueDate::DayTime(NaiveDateTime::new(due_naive_date, time)),
        None => DueDate::Day(due_naive_date),
    };
    task.due_date = Some(due_date);

    Ok(task)
}
#[cfg(test)]
mod test {

    use chrono::{Datelike, Days, NaiveDate, NaiveDateTime, NaiveTime};

    use crate::{config::Config, parser::parser_task::*, task::TaskState};

    #[test]
    fn test_parse_task_no_description() {
        let mut input = "- [x] 10/15 task_name #done";
        let config = Config::default();
        let res = parse_task(&mut input, &config);
        assert!(res.is_ok());
        let res = res.unwrap();
        let year = chrono::Local::now().year();
        let expected = Task {
            name: "task_name".to_string(),
            description: None,
            tags: Some(vec!["done".to_string()]),
            due_date: Some(DueDate::Day(NaiveDate::from_ymd_opt(year, 10, 15).unwrap())),
            priority: 0,
            state: TaskState::Done,
        };
        assert_eq!(res, expected);
    }

    #[test]
    fn test_parse_task_only_state() {
        let mut input = "- [ ]";
        let config = Config::default();
        let res = parse_task(&mut input, &config);
        assert!(res.is_ok());
        let res = res.unwrap();
        let now = chrono::Local::now();
        let expected = Task {
            name: "New Task".to_string(),
            description: None,
            tags: None,
            due_date: Some(DueDate::Day(now.date_naive())),
            priority: 0,
            state: TaskState::ToDo,
        };
        assert_eq!(res, expected);
    }
    #[test]
    fn test_parse_task_with_due_date_words() {
        let mut input = "- [ ] today 15:30 task_name";
        let config = Config::default();
        let res = parse_task(&mut input, &config);
        assert!(res.is_ok());
        let res = res.unwrap();
        let expected_date = chrono::Local::now().date_naive();
        let expected_time = NaiveTime::from_hms_opt(15, 30, 0).unwrap();
        let expected_due_date = DueDate::DayTime(NaiveDateTime::new(expected_date, expected_time));
        assert_eq!(res.due_date, Some(expected_due_date));
    }

    #[test]
    fn test_parse_task_with_weekday() {
        let mut input = "- [ ] monday 15:30 task_name";
        let config = Config::default();
        let res = parse_task(&mut input, &config);
        assert!(res.is_ok());
        let res = res.unwrap();

        let now = chrono::Local::now();
        let expected_date = now
            .date_naive()
            .checked_add_days(Days::new(
                8 - now.date_naive().weekday().number_from_monday() as u64,
            ))
            .unwrap();
        let expected_time = NaiveTime::from_hms_opt(15, 30, 0).unwrap();
        let expected_due_date = DueDate::DayTime(NaiveDateTime::new(expected_date, expected_time));
        assert_eq!(res.due_date, Some(expected_due_date));
    }

    #[test]
    fn test_parse_task_with_weekday_this() {
        let mut input = "- [ ] this monday 15:30 task_name";
        let config = Config::default();
        let res = parse_task(&mut input, &config);
        assert!(res.is_ok());
        let res = res.unwrap();
        let now = chrono::Local::now();
        let expected_date = now
            .date_naive()
            .checked_add_days(Days::new(
                8 - now.date_naive().weekday().number_from_monday() as u64,
            ))
            .unwrap();
        let expected_time = NaiveTime::from_hms_opt(15, 30, 0).unwrap();
        let expected_due_date = DueDate::DayTime(NaiveDateTime::new(expected_date, expected_time));
        assert_eq!(res.due_date, Some(expected_due_date));
    }

    #[test]
    fn test_parse_task_with_weekday_next() {
        let mut input = "- [ ] next monday 15:30 task_name";
        let config = Config::default();
        let res = parse_task(&mut input, &config);
        assert!(res.is_ok());
        let res = res.unwrap();
        let now = chrono::Local::now();
        let expected_date = now
            .date_naive()
            .checked_add_days(Days::new(
                8 - now.date_naive().weekday().number_from_monday() as u64,
            ))
            .unwrap();
        let expected_time = NaiveTime::from_hms_opt(15, 30, 0).unwrap();
        let expected_due_date = DueDate::DayTime(NaiveDateTime::new(expected_date, expected_time));
        assert_eq!(res.due_date, Some(expected_due_date));
    }

    #[test]
    fn test_parse_task_without_due_date() {
        let mut input = "- [ ] task_name";
        let config = Config::default();
        let res = parse_task(&mut input, &config);
        assert!(res.is_ok());
        let res = res.unwrap();
        let expected_due_date = DueDate::Day(chrono::Local::now().date_naive());
        assert_eq!(res.due_date, Some(expected_due_date));
    }

    #[test]
    fn test_parse_task_with_invalid_state() {
        let mut input = "- [invalid] task_name";
        let config = Config::default();
        let res = parse_task(&mut input, &config);
        assert!(res.is_err());
    }

    #[test]
    fn test_parse_task_without_state() {
        let mut input = "task_name";
        let config = Config::default();
        let res = parse_task(&mut input, &config);
        assert!(res.is_err());
    }

    #[test]
    fn test_parse_task_with_invalid_priority() {
        let mut input = "- [ ] task_name p-9";
        let config = Config::default();
        let res = parse_task(&mut input, &config);
        assert!(res.is_ok());
        let res = res.unwrap();
        assert_eq!(res.priority, 0); // Default priority is used when the provided one is invalid
    }

    #[test]
    fn test_parse_task_without_name() {
        let mut input = "- [ ]";
        let config = Config::default();
        let res = parse_task(&mut input, &config);
        assert!(res.is_ok());
        let res = res.unwrap();
        assert_eq!(res.name, "New Task"); // Default name is used when no name is provided
    }
}
