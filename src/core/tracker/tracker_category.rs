#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoreEntry {
    pub score: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoolEntry {
    pub value: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryTypeVariant {
    Score,
    Bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryType {
    Score(ScoreEntry),
    Bool(BoolEntry),
}

impl EntryType {
    fn variant(&self) -> EntryTypeVariant {
        match self {
            EntryType::Score(_) => EntryTypeVariant::Score,
            EntryType::Bool(_) => EntryTypeVariant::Bool,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TrackerEntry {
    /// There is always a notes field in a tracker category
    note: String,
    /// Entry
    entry: EntryType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackerCategory {
    /// Name of the tracker category
    name: String,
    /// The type of entries this category accepts
    entry_type: EntryTypeVariant,
    /// Entries in this tracker category
    entries: Vec<TrackerEntry>,
}
