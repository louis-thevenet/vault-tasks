use crossterm::event::KeyEvent;
use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize, Hash)]
pub enum Action {
    Tick,
    Render,
    Resize(u16, u16),
    Suspend,
    Resume,
    Quit,
    ClearScreen,
    Error(String),
    Help,
    // Raw Key Events
    Key(KeyEvent),
    ReloadVault,
    // Movements
    Up,
    Down,
    Left,
    Right,
    Enter,
    Cancel,
    // View
    ViewPageUp,
    ViewUp,
    ViewPageDown,
    ViewDown,
    ViewLeft,
    ViewRight,
    // Menus
    Escape,
    Search,
    TabRight,
    TabLeft,
    Open,
    EditTask,
    FocusExplorer,
    FocusFilter,
}
impl PartialOrd for Action {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Action {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.to_string().cmp(&other.to_string())
    }
}
