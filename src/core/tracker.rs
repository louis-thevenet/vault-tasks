use tracker_category::TrackerCategory;

use super::date::Date;
mod tracker_category;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tracker {
    /// Name of the tracker
    name: String,
    /// Date of the first occurrence
    start_date: Date,
    /// Frequency in seconds, (`start_date` + `frequency` * `n` = n-th occurrence)
    frequency: usize,
    /// Categories of the tracker
    categories: Vec<TrackerCategory>,
}
