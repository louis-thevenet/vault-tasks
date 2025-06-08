use std::fmt::Display;
use chrono::{Datelike, NaiveDate, NaiveDateTime, TimeDelta};
use frequency::Frequency;
use tabled::{builder::Builder, settings::Style};
use tracker_category::{EntryType, TrackerCategory};

use super::date::Date;
mod frequency;
mod tracker_category;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tracker {
    /// Name of the tracker
    name: String,
    /// Date of the first occurrence
    start_date: Date,
    /// Total number of occurrences
    length: usize,
    /// Frequency (`start_date` + `frequency` * `n` = n-th occurrence)
    frequency: Frequency,
    /// Categories of the tracker
    categories: Vec<TrackerCategory>,
    /// Additional notes for the tracker
    notes: Vec<String>,
}
impl Tracker {
    pub(crate) fn test() -> Tracker {
        Tracker {
            name: "Test Tracker".to_string(),
            start_date: Date::Day(NaiveDate::from_ymd_opt(2022, 1, 1).unwrap()),
            length: 5,
            frequency: Frequency::EveryXDays(5),
            categories: vec![
                TrackerCategory {
                    name: "Some score stuff".to_string(),
                    entries: vec![
                        EntryType::Score(tracker_category::ScoreEntry { score: 4 }),
                        EntryType::Score(tracker_category::ScoreEntry { score: 5 }),
                        EntryType::Score(tracker_category::ScoreEntry { score: 5 }),
                        EntryType::Score(tracker_category::ScoreEntry { score: 2 }),
                        EntryType::Score(tracker_category::ScoreEntry { score: 5 }),
                    ],
                },
                TrackerCategory {
                    name: "Some boolean stuff".to_string(),
                    entries: vec![
                        EntryType::Bool(tracker_category::BoolEntry { value: true }),
                        EntryType::Bool(tracker_category::BoolEntry { value: true }),
                        EntryType::Bool(tracker_category::BoolEntry { value: true }),
                        EntryType::Bool(tracker_category::BoolEntry { value: false }),
                        EntryType::Bool(tracker_category::BoolEntry { value: true }),
                    ],
                },
            ],
            notes: vec![
                "This is a test tracker.".to_string(),
                "Created for demonstration purposes.".to_string(),
                String::new(),
                "It has two categories with different entry types.".to_string(),
                String::new(),
            ],
        }
    }
}

impl Display for Tracker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut b = Builder::new();
        b.push_record(
            [
                vec![self.frequency.to_string()],
                self.categories.iter().map(|c| c.name.clone()).collect(),
                vec!["Notes".to_string()],
            ]
            .concat(),
        );
        let mut date = self.start_date.clone();
        for n in 0..self.length {
            b.push_record(
                [
                    vec![date.to_string()],
                    self.categories
                        .iter()
                        .map(|c| c.entries.get(n).unwrap().to_string())
                        .collect(),
                    vec![self.notes.get(n).map_or("", |v| v).to_string()],
                ]
                .concat(),
            );
            date = self.frequency.next_date(&date);
        }

        writeln!(f, "Tracker: {}", self.name)?;
        write!(f, "{}", b.build().with(Style::markdown()))
    }
}
