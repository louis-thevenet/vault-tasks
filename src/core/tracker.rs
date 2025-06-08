use std::fmt::Display;

use frequency::Frequency;
use tabled::{builder::Builder, settings::Style};
use tracker_category::TrackerCategory;

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
}

impl Display for Tracker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut b = Builder::new();
        b.push_record(
            vec![
                self.categories.iter().map(|c| c.name.clone()).collect(),
                vec!["Notes".to_string()],
            ]
            .concat(),
        );

        for n in 0..self.length {
            b.push_record(
                self.categories
                    .iter()
                    .map(|c| c.entries.get(n).unwrap().to_string()),
            );
        }

        write!(f, "{}", b.build().with(Style::markdown()).to_string())
    }
}
