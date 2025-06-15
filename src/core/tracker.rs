use chrono::NaiveDateTime;
use color_eyre::{Result, eyre::bail};
use frequency::Frequency;
use std::fs::read_to_string;
use std::io::Write;
use std::{fmt::Display, fs::File, path};
use tabled::{builder::Builder, settings::Style};
use tracker_category::{NoteEntry, TrackerCategory, TrackerEntry};

use super::TasksConfig;
use super::date::Date;

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
            start_date: frequency.fix_date(&self.start_date),
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
    pub fn add_event(&mut self, date: &Date, entries: &[TrackerEntry]) {
        // What date is it supposed to be?
        let parsed_entries_count = self.categories.first().map_or(0, |c| c.entries.len());
        let mut next_date = self.start_date.clone();
        for _ in 0..parsed_entries_count {
            next_date = self.frequency.next_date(&next_date);
        }
        // If a date was skipped,
        // we'll push the missing dates
        // it's expensive to count like this but you should have not skipped dates >:(
        while date > &next_date {
            self.categories.iter_mut().for_each(|cat| {
                cat.entries.push(TrackerEntry::Blank);
            });
            self.length += 1;
            next_date = self.frequency.next_date(&next_date);
        }

        let entries_iter = entries.iter();
        self.categories
            .iter_mut()
            .zip(entries_iter.clone()) // will consume only the correct amount
            .for_each(|(cat, entry)| {
                cat.entries.push(entry.clone());
            });
        self.length += 1;
    }
    fn fmt(&self, american_format: bool) -> String {
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
                    vec![date.to_string_format(!american_format)],
                    self.categories
                        .iter()
                        .map(|c| c.entries.get(n).unwrap().to_string())
                        .collect(),
                ]
                .concat(),
            );
            date = self.frequency.next_date(&date);
        }

        [
            format!(
                "Tracker: {} ({})\n",
                self.name,
                self.start_date.to_string_format(!american_format)
            ),
            format!("{}", b.build().with(Style::markdown())),
        ]
        .join("\n")
    }

    /// Replaces the input tracker with the parsed and fixed version.
    pub(crate) fn fix_tracker_attributes(
        &self,
        config: &TasksConfig,
        path: &path::Path,
    ) -> Result<()> {
        if !path.is_file() {
            bail!("Tried to fix tasks attributes but {path:?} is not a file");
        }
        let new_lines = self
            .fmt(config.use_american_format)
            .split('\n')
            .map(str::to_owned)
            .collect::<Vec<String>>();
        let content = read_to_string(path)?;
        let mut lines = content
            .split('\n')
            .map(str::to_owned)
            .collect::<Vec<String>>();
        for (n, new_line) in new_lines.iter().enumerate() {
            if lines.len() <= self.line_number + n {
                lines.push(new_line.to_string());
            } else {
                lines[n + self.line_number] = new_line.to_string();
            }
        }

        let mut file = File::create(path)?;
        file.write_all(lines.join("\n").as_bytes())?;
        Ok(())
    }

    pub fn add_blanks(&self, config: &TasksConfig) -> Tracker {
        let mut fixed_tracker = self.clone();
        let entries_count = fixed_tracker
            .categories
            .first()
            .map_or(0, |c| c.entries.len());

        // Compute last parsed entry's date
        let mut last_parsed_date = fixed_tracker.start_date.clone();
        (0..entries_count)
            .for_each(|_| last_parsed_date = fixed_tracker.frequency.next_date(&last_parsed_date));

        let now = chrono::Utc::now();
        let mut target_date = Date::DayTime(NaiveDateTime::new(now.date_naive(), now.time()));
        // increment target_date so we add extra blanks
        (0..config.tracker_extra_blanks)
            .for_each(|_| target_date = fixed_tracker.frequency.next_date(&target_date));

        // Add blanks until we reach the current date
        while target_date >= last_parsed_date {
            fixed_tracker.categories.iter_mut().for_each(|cat| {
                cat.entries.push(TrackerEntry::Blank);
            });
            fixed_tracker.length += 1;
            last_parsed_date = fixed_tracker.frequency.next_date(&last_parsed_date);
        }

        // Now we'll ensure we have enough blanks at the end
        let mut blanks_count = 0;
        for _i in 0..config.tracker_extra_blanks {
            if fixed_tracker.categories.iter().all(|cat| {
                cat.entries
                    .get(cat.entries.len() - 1 - blanks_count)
                    .is_some_and(|entry| match entry {
                        TrackerEntry::Note(NoteEntry { value }) if value.is_empty() => true,
                        TrackerEntry::Blank => true,
                        _ => false,
                    })
            }) {
                blanks_count += 1;
            } else {
                break;
            }
        }

        // Add missing blanks
        for _i in 0..(config.tracker_extra_blanks - blanks_count) {
            for cat in &mut fixed_tracker.categories {
                cat.entries.push(TrackerEntry::Blank);
            }
            fixed_tracker.length += 1;
        }
        fixed_tracker
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
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use frequency::Frequency;
    use tracker_category::{NoteEntry, TrackerCategory, TrackerEntry};

    fn create_test_tracker() -> Tracker {
        Tracker {
            name: "Test Tracker".to_string(),
            filename: "test.md".to_string(),
            line_number: 1,
            start_date: Date::Day(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
            length: 0,
            frequency: Frequency::Days(1),
            categories: vec![
                TrackerCategory {
                    name: "Category 1".to_string(),
                    entries: vec![],
                },
                TrackerCategory {
                    name: "Category 2".to_string(),
                    entries: vec![],
                },
            ],
        }
    }

    #[test]
    fn test_add_event_no_skipped_dates() {
        let mut tracker = create_test_tracker();
        let date = Date::Day(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()); // Same as start_date
        let entries = vec![
            TrackerEntry::Note(NoteEntry {
                value: "First entry".to_string(),
            }),
            TrackerEntry::Note(NoteEntry {
                value: "Second entry".to_string(),
            }),
        ];

        tracker.add_event(&date, &entries);

        assert_eq!(tracker.length, 1);
        assert_eq!(tracker.categories[0].entries.len(), 1);
        assert_eq!(tracker.categories[1].entries.len(), 1);

        // Verify the actual entries were added correctly
        match &tracker.categories[0].entries[0] {
            TrackerEntry::Note(note) => assert_eq!(note.value, "First entry"),
            _ => panic!("Expected Note entry"),
        }
        match &tracker.categories[1].entries[0] {
            TrackerEntry::Note(note) => assert_eq!(note.value, "Second entry"),
            _ => panic!("Expected Note entry"),
        }
    }

    #[test]
    fn test_add_event_skip_one_date_daily() {
        let mut tracker = create_test_tracker();

        // Add first event on start date
        let first_date = Date::Day(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        let first_entries = vec![
            TrackerEntry::Note(NoteEntry {
                value: "Day 1".to_string(),
            }),
            TrackerEntry::Note(NoteEntry {
                value: "Day 1 Cat 2".to_string(),
            }),
        ];
        tracker.add_event(&first_date, &first_entries);

        // Skip January 2nd and add event for January 3rd
        let skipped_date = Date::Day(NaiveDate::from_ymd_opt(2024, 1, 3).unwrap());
        let skipped_entries = vec![
            TrackerEntry::Note(NoteEntry {
                value: "Day 3".to_string(),
            }),
            TrackerEntry::Note(NoteEntry {
                value: "Day 3 Cat 2".to_string(),
            }),
        ];

        tracker.add_event(&skipped_date, &skipped_entries);

        // Should have 3 total entries now (Day 1, Blank for Day 2, Day 3)
        assert_eq!(tracker.length, 3);
        assert_eq!(tracker.categories[0].entries.len(), 3);
        assert_eq!(tracker.categories[1].entries.len(), 3);

        // Verify Day 1 entries
        match &tracker.categories[0].entries[0] {
            TrackerEntry::Note(note) => assert_eq!(note.value, "Day 1"),
            _ => panic!("Expected Note entry for Day 1"),
        }

        // Verify Day 2 (skipped) entries are blank
        assert_eq!(tracker.categories[0].entries[1], TrackerEntry::Blank);
        assert_eq!(tracker.categories[1].entries[1], TrackerEntry::Blank);

        // Verify Day 3 entries
        match &tracker.categories[0].entries[2] {
            TrackerEntry::Note(note) => assert_eq!(note.value, "Day 3"),
            _ => panic!("Expected Note entry for Day 3"),
        }
    }

    #[test]
    fn test_add_event_skip_multiple_dates() {
        let mut tracker = create_test_tracker();

        // Add first event
        let first_date = Date::Day(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        let first_entries = vec![
            TrackerEntry::Note(NoteEntry {
                value: "Day 1".to_string(),
            }),
            TrackerEntry::Note(NoteEntry {
                value: "Day 1 Cat 2".to_string(),
            }),
        ];
        tracker.add_event(&first_date, &first_entries);

        // Skip 3 days (Jan 2, 3, 4) and add event for January 5th
        let skipped_date = Date::Day(NaiveDate::from_ymd_opt(2024, 1, 5).unwrap());
        let skipped_entries = vec![
            TrackerEntry::Note(NoteEntry {
                value: "Day 5".to_string(),
            }),
            TrackerEntry::Note(NoteEntry {
                value: "Day 5 Cat 2".to_string(),
            }),
        ];

        tracker.add_event(&skipped_date, &skipped_entries);

        // Should have 5 total entries now (Day 1, 3 blanks, Day 5)
        assert_eq!(tracker.length, 5);
        assert_eq!(tracker.categories[0].entries.len(), 5);
        assert_eq!(tracker.categories[1].entries.len(), 5);

        // Verify Day 1 entries
        match &tracker.categories[0].entries[0] {
            TrackerEntry::Note(note) => assert_eq!(note.value, "Day 1"),
            _ => panic!("Expected Note entry for Day 1"),
        }

        // Verify skipped days (indices 1, 2, 3) are blank
        for i in 1..4 {
            assert_eq!(tracker.categories[0].entries[i], TrackerEntry::Blank);
            assert_eq!(tracker.categories[1].entries[i], TrackerEntry::Blank);
        }

        // Verify Day 5 entries
        match &tracker.categories[0].entries[4] {
            TrackerEntry::Note(note) => assert_eq!(note.value, "Day 5"),
            _ => panic!("Expected Note entry for Day 5"),
        }
    }

    #[test]
    fn test_add_event_weekly_frequency_skip_dates() {
        let mut tracker = Tracker {
            name: "Weekly Tracker".to_string(),
            filename: "test.md".to_string(),
            line_number: 1,
            start_date: Date::Day(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()), // Monday
            length: 0,
            frequency: Frequency::Days(7),
            categories: vec![TrackerCategory {
                name: "Weekly Cat".to_string(),
                entries: vec![],
            }],
        };

        // Add first week
        let first_week = Date::Day(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        let first_entries = vec![TrackerEntry::Note(NoteEntry {
            value: "Week 1".to_string(),
        })];
        tracker.add_event(&first_week, &first_entries);

        // Skip week 2 (Jan 8) and add week 3 (Jan 15)
        let third_week = Date::Day(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
        let third_entries = vec![TrackerEntry::Note(NoteEntry {
            value: "Week 3".to_string(),
        })];
        tracker.add_event(&third_week, &third_entries);

        // Should have 3 entries (Week 1, Blank for Week 2, Week 3)
        assert_eq!(tracker.length, 3);
        assert_eq!(tracker.categories[0].entries.len(), 3);

        // Verify entries
        match &tracker.categories[0].entries[0] {
            TrackerEntry::Note(note) => assert_eq!(note.value, "Week 1"),
            _ => panic!("Expected Note entry for Week 1"),
        }

        assert_eq!(tracker.categories[0].entries[1], TrackerEntry::Blank);

        match &tracker.categories[0].entries[2] {
            TrackerEntry::Note(note) => assert_eq!(note.value, "Week 3"),
            _ => panic!("Expected Note entry for Week 3"),
        }
    }

    #[test]
    fn test_add_event_skip_dates_with_multiple_categories() {
        let mut tracker = Tracker {
            name: "Multi Category Tracker".to_string(),
            filename: "test.md".to_string(),
            line_number: 1,
            start_date: Date::Day(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
            length: 0,
            frequency: Frequency::Days(1),
            categories: vec![
                TrackerCategory {
                    name: "Cat A".to_string(),
                    entries: vec![],
                },
                TrackerCategory {
                    name: "Cat B".to_string(),
                    entries: vec![],
                },
                TrackerCategory {
                    name: "Cat C".to_string(),
                    entries: vec![],
                },
            ],
        };

        // Add first event
        let first_date = Date::Day(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        let first_entries = vec![
            TrackerEntry::Note(NoteEntry {
                value: "A1".to_string(),
            }),
            TrackerEntry::Note(NoteEntry {
                value: "B1".to_string(),
            }),
            TrackerEntry::Note(NoteEntry {
                value: "C1".to_string(),
            }),
        ];
        tracker.add_event(&first_date, &first_entries);

        // Skip one day and add third day
        let third_date = Date::Day(NaiveDate::from_ymd_opt(2024, 1, 3).unwrap());
        let third_entries = vec![
            TrackerEntry::Note(NoteEntry {
                value: "A3".to_string(),
            }),
            TrackerEntry::Note(NoteEntry {
                value: "B3".to_string(),
            }),
            TrackerEntry::Note(NoteEntry {
                value: "C3".to_string(),
            }),
        ];
        tracker.add_event(&third_date, &third_entries);

        // Verify all categories have the same structure
        for category in &tracker.categories {
            assert_eq!(category.entries.len(), 3);
            assert_eq!(category.entries[1], TrackerEntry::Blank);
        }

        // Verify specific entries
        match &tracker.categories[0].entries[0] {
            TrackerEntry::Note(note) => assert_eq!(note.value, "A1"),
            _ => panic!("Expected Note entry"),
        }
        match &tracker.categories[2].entries[2] {
            TrackerEntry::Note(note) => assert_eq!(note.value, "C3"),
            _ => panic!("Expected Note entry"),
        }
    }

    #[test]
    fn test_add_event_sequential_no_skips() {
        let mut tracker = create_test_tracker();

        // Add events sequentially without skipping
        for day in 1..=5 {
            let date = Date::Day(NaiveDate::from_ymd_opt(2024, 1, day).unwrap());
            let entries = vec![
                TrackerEntry::Note(NoteEntry {
                    value: format!("Day {day} Cat 1"),
                }),
                TrackerEntry::Note(NoteEntry {
                    value: format!("Day {day} Cat 2"),
                }),
            ];
            tracker.add_event(&date, &entries);
        }

        // Should have exactly 5 entries with no blanks
        assert_eq!(tracker.length, 5);
        assert_eq!(tracker.categories[0].entries.len(), 5);
        assert_eq!(tracker.categories[1].entries.len(), 5);

        // Verify no blank entries were created
        for category in &tracker.categories {
            for entry in &category.entries {
                assert_ne!(*entry, TrackerEntry::Blank);
            }
        }
    }

    #[test]
    fn test_add_event_skip_with_existing_entries() {
        let mut tracker = create_test_tracker();

        // Add several sequential events first
        for day in 1..=3 {
            let date = Date::Day(NaiveDate::from_ymd_opt(2024, 1, day).unwrap());
            let entries = vec![
                TrackerEntry::Note(NoteEntry {
                    value: format!("Day {day}"),
                }),
                TrackerEntry::Note(NoteEntry {
                    value: format!("Cat2 Day {day}"),
                }),
            ];
            tracker.add_event(&date, &entries);
        }

        // Now skip a few days and add another event
        let skipped_date = Date::Day(NaiveDate::from_ymd_opt(2024, 1, 7).unwrap()); // Skip days 4, 5, 6
        let skipped_entries = vec![
            TrackerEntry::Note(NoteEntry {
                value: "Day 7".to_string(),
            }),
            TrackerEntry::Note(NoteEntry {
                value: "Cat2 Day 7".to_string(),
            }),
        ];
        tracker.add_event(&skipped_date, &skipped_entries);

        // Should have 7 entries total (3 initial + 3 blanks + 1 new)
        assert_eq!(tracker.length, 7);
        assert_eq!(tracker.categories[0].entries.len(), 7);

        // Verify the structure: Days 1-3 should be filled, days 4-6 should be blank, day 7 should be filled
        for i in 0..3 {
            match &tracker.categories[0].entries[i] {
                TrackerEntry::Note(note) => assert_eq!(note.value, format!("Day {}", i + 1)),
                _ => panic!("Expected Note entry for day {}", i + 1),
            }
        }

        // Days 4-6 should be blank (indices 3-5)
        for i in 3..6 {
            assert_eq!(tracker.categories[0].entries[i], TrackerEntry::Blank);
            assert_eq!(tracker.categories[1].entries[i], TrackerEntry::Blank);
        }

        // Day 7 should be filled (index 6)
        match &tracker.categories[0].entries[6] {
            TrackerEntry::Note(note) => assert_eq!(note.value, "Day 7"),
            _ => panic!("Expected Note entry for day 7"),
        }
    }
}
