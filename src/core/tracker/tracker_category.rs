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
pub struct NoteEntry {
    pub value: String,
}

impl Display for NoteEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackerEntry {
    Score(ScoreEntry),
    Bool(BoolEntry),
    Note(NoteEntry),
    Blank,
}

impl Display for TrackerEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrackerEntry::Score(entry) => write!(f, "{entry}",),
            TrackerEntry::Bool(entry) => write!(f, "[{entry}]",),
            TrackerEntry::Note(entry) => write!(f, "{entry}"),
            TrackerEntry::Blank => write!(f, ""),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackerCategory {
    /// Name of the tracker category
    pub name: String,
    /// Entries in this tracker category
    pub entries: Vec<TrackerEntry>,
}
