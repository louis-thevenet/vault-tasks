use chrono::{Datelike, NaiveDate, NaiveDateTime, TimeDelta};
use frequency::Frequency;
use std::fmt::Display;
use tabled::{builder::Builder, settings::Style};
use tracker_category::{EntryType, TrackerCategory};

use super::date::Date;
pub mod frequency;
mod tracker_category;

pub struct NewTracker {
    pub name: String,
}

impl NewTracker {
    pub fn new(name: String) -> Self {
        Self { name }
    }
    pub fn to_incomplete_tracker(
        self,

        frequency: Frequency,
        categories: Vec<String>,
    ) -> IncompleteTracker {
        IncompleteTracker {
            name: self.name,
            frequency,
            categories,
        }
    }
}
pub struct IncompleteTracker {
    pub name: String,
    frequency: Frequency,
    categories: Vec<String>,
}
impl IncompleteTracker {
    pub fn complete(
        self,
        start_date: Date,
        length: usize,
        categories: Vec<TrackerCategory>,
        notes: Vec<String>,
    ) -> Tracker {
        Tracker {
            name: self.name,
            start_date,
            length,
            frequency: self.frequency,
            categories,
            notes,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tracker {
    /// Name of the tracker
    pub name: String,
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
    pub fn add_event(&mut self, date: &Date, entries: Vec<EntryType>) {
        // should ensure date is valid
        self.categories
            .iter_mut()
            .zip(entries.iter())
            .map(|(cat, entry)| {
                cat.entries.push(entry.clone());
            });
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
