use frequency::Frequency;
use std::{fmt::Display, path};
use tabled::{builder::Builder, settings::Style};
use tracing::{debug, error};
use tracker_category::{NoteEntry, TrackerCategory, TrackerEntry};

use super::{TasksConfig, date::Date};
pub mod frequency;
pub mod tracker_category;

/// We need this state because of how the Tracker is parsed.
///
/// <!-- Tracker: tracker name <started on> -->
/// <!-- | frequency | tracker categories | ... | notes | -->
/// <!-- | --------- | ------------------ | --- | ----- | -->
/// <!-- | date      | x                  | ... |  note | -->
///
pub struct NewTracker {
    pub name: String,
    pub start_date: Date,
}

impl NewTracker {
    pub fn new(name: String, start_date: Date) -> Self {
        Self { name, start_date }
    }
    /// Converts the `NewTracker` into a `Tracker` which has no entry.
    pub fn to_tracker(&self, frequency: Frequency, categories: Vec<String>) -> Tracker {
        let tracker_categories = categories
            .into_iter()
            .map(|name| TrackerCategory {
                name,
                entries: vec![],
            })
            .collect::<Vec<TrackerCategory>>();
        Tracker {
            name: self.name.clone(),
            frequency,
            categories: tracker_categories,
            start_date: self.start_date.clone(),
            length: 0,
            notes: vec![],
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
    pub categories: Vec<TrackerCategory>,
    /// Additional notes for the tracker
    notes: Vec<String>,
}
impl Tracker {
    pub fn add_event(&mut self, _date: &Date, entries: &[TrackerEntry]) {
        // should ensure date is valid
        let entries_iter = entries.iter();
        self.categories
            .iter_mut()
            .zip(entries_iter.clone()) // will consume only the correct amount
            .for_each(|(cat, entry)| {
                cat.entries.push(entry.clone());
            });
        if entries_iter.clone().count() > self.categories.len() {
            if let Some(TrackerEntry::Note(NoteEntry { value: note })) = entries_iter.last() {
                self.notes.push(note.to_string());
            } else {
                error!("Last element of TrackerEntry entries was not a note but was still extra.");
            }
        }
        self.length += 1;
    }

    pub(crate) fn fix_tracker_attributes(&self, _config: &TasksConfig, _filename: &path::Path) {
        debug!("Fixing Tracker attributes (not yet implemented)");
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
