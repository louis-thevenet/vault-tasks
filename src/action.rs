use crossterm::event::KeyEvent;
use serde::{Deserialize, Serialize};
use strum::Display;

use crate::app::Mode;

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
    PreviousMethod,
    NextMethod,
    NextSegment,
    Pause,
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
    SwitchSortingMode,
    Escape,
    Search,
    TabRight,
    TabLeft,
    Open,
    Edit,
    ToggleStatus,
    Focus(Mode),
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
