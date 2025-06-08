use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoreEntry {
    pub score: i32,
}
impl Display for ScoreEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.score)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoolEntry {
    pub value: bool,
}

impl Display for BoolEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryType {
    Score(ScoreEntry),
    Bool(BoolEntry),
}

impl Display for EntryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntryType::Score(entry) => write!(f, "{}", entry),
            EntryType::Bool(entry) => write!(f, "{}", entry),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackerCategory {
    /// Name of the tracker category
    pub name: String,
    /// Entries in this tracker category
    pub entries: Vec<EntryType>,
}
