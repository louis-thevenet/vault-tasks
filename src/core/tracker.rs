use color_eyre::{Result, eyre::bail};
use frequency::Frequency;
use std::fs::read_to_string;
use std::io::Write;
use std::{fmt::Display, fs::File, path};
use tabled::{builder::Builder, settings::Style};
use tracing::Instrument;
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
    pub fn to_tracker(
        &self,
        filename: String,
        line_number: usize,
        frequency: Frequency,
        categories: Vec<String>,
    ) -> Tracker {
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
            filename,
            line_number,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tracker {
    /// Name of the tracker
    pub name: String,
    /// Filename
    pub filename: String,
    /// Line number in the file where the tracker is defined
    pub line_number: usize,
    /// Date of the first occurrence
    pub start_date: Date,
    /// Total number of occurrences
    pub length: usize,
    /// Frequency (`start_date` + `frequency` * `n` = n-th occurrence)
    pub frequency: Frequency,
    /// Categories of the tracker
    pub categories: Vec<TrackerCategory>,
}
impl Tracker {
    /// Amount of blanks to create when fixing the tracker attributes.
    const BLANKS_COUNT: usize = 3;

    pub fn add_event(&mut self, _date: &Date, entries: &[TrackerEntry]) {
        // should ensure date is valid
        let entries_iter = entries.iter();
        self.categories
            .iter_mut()
            .zip(entries_iter.clone()) // will consume only the correct amount
            .for_each(|(cat, entry)| {
                cat.entries.push(entry.clone());
            });
        self.length += 1;
    }
    pub(crate) fn fix_tracker_attributes(
        &self,
        _config: &TasksConfig,
        path: &path::Path,
    ) -> Result<()> {
        if !path.is_file() {
            bail!("Tried to fix tasks attributes but {path:?} is not a file");
        }
        let content = read_to_string(path)?;
        let mut lines = content
            .split('\n')
            .map(str::to_owned)
            .collect::<Vec<String>>();
        let mut fixed_tracker = self.clone();
        let mut count = 0;
        for _i in 0..Self::BLANKS_COUNT {
            if fixed_tracker.categories.iter().all(|cat| {
                cat.entries
                    .get(cat.entries.len() - 1 - count)
                    .is_some_and(|entry| match entry {
                        TrackerEntry::Note(NoteEntry { value }) if value.is_empty() => true,
                        TrackerEntry::Blank => true,
                        _ => false,
                    })
            }) {
                count += 1;
            } else {
                break;
            }
        }

        for _i in 0..(Self::BLANKS_COUNT - count) {
            for cat in &mut fixed_tracker.categories {
                cat.entries.push(TrackerEntry::Blank);
            }
            fixed_tracker.length += 1;
        }
        let new_lines = fixed_tracker
            .to_string()
            .split('\n')
            .map(str::to_owned)
            .collect::<Vec<String>>();

        for (n, new_line) in new_lines.iter().enumerate() {
            if lines.len() <= fixed_tracker.line_number + n {
                lines.push(new_line.to_string());
            } else {
                lines[n + fixed_tracker.line_number] = new_line.to_string();
            }
        }

        let mut file = File::create(path)?;
        file.write_all(lines.join("\n").as_bytes())?;
        Ok(())
    }
}

impl Display for Tracker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut b = Builder::new();
        b.push_record(
            [
                vec![self.frequency.to_string()],
                self.categories.iter().map(|c| c.name.clone()).collect(),
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
                ]
                .concat(),
            );
            date = self.frequency.next_date(&date);
        }

        writeln!(f, "Tracker: {} ({})\n", self.name, self.start_date)?;
        write!(f, "{}", b.build().with(Style::markdown()))
    }
}
