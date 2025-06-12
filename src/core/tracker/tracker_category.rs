use std::fmt::Display;

use crate::core::PrettySymbolsConfig;

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
        write!(f, "{}", if self.value { "x" } else { " " })
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
impl TrackerEntry {
    pub fn pretty_fmt(&self, pretty_symbols: &PrettySymbolsConfig) -> String {
        match self {
            TrackerEntry::Score(score_entry) => score_entry.to_string(),
            TrackerEntry::Bool(bool_entry) => {
                if bool_entry.value {
                    pretty_symbols.task_done.clone()
                } else {
                    pretty_symbols.task_todo.clone()
                }
            }
            TrackerEntry::Note(note_entry) => note_entry.to_string(),
            TrackerEntry::Blank => String::new(),
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
