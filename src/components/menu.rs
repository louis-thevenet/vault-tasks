use color_eyre::Result;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::{action::Action, config::Config};

#[derive(Default)]
pub struct Menu {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
}
impl Menu {
    pub fn new() -> Self {
        Self::default()
    }
}
impl Component for Menu {
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let surrounding_block = Block::default().borders(Borders::ALL).title("Lateral Menu");
        let items = ["Item 1", "Item 2", "Item 3"];
        let list = List::new(items).block(surrounding_block);

        frame.render_widget(list, area);

        Ok(())
    }
}
