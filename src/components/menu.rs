use color_eyre::Result;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List},
};

use crate::task_core::TaskManager;

use super::Component;

#[derive(Default)]
pub struct Menu {
    // command_tx: Option<UnboundedSender<Action>>,
    // config: Config,
    task_mgr: TaskManager,
    // selected_index: usize,
    selected_header_path: Vec<String>,
}
impl Menu {
    pub fn new() -> Self {
        Self::default()
    }
}
impl Component for Menu {
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let surrounding_block = Block::default().borders(Borders::ALL).title("Lateral Menu");
        let items = match self.task_mgr.get_entries(self.selected_header_path.clone()) {
            Ok(items) => items,
            Err(e) => vec![e.to_string()],
        };

        let list = List::new(items).block(surrounding_block);

        frame.render_widget(list, area);

        Ok(())
    }
}
