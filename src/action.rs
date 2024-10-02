use crossterm::event::KeyEvent;
use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
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
    Key(KeyEvent),
    Up,
    Down,
    Left,
    Right,
    Enter,
    Cancel,
    Search,
    TabRight,
    TabLeft,
    FocusExplorer,
    FocusFilter,
}
