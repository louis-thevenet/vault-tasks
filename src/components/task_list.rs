use color_eyre::Result;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;
use tracing::info;

use super::Component;
use crate::{action::Action, config::Config};

#[derive(Default)]
pub struct TaskList {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
}
impl TaskList {
    pub fn new() -> Self {
        Self::default()
    }
}
impl Component for TaskList {
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let surrounding_block = Block::default()
            .borders(Borders::ALL)
            .title("Here is a list of tasks");
        let items = ["Task 1", "Task 2", "Task 3"];
        let list = List::new(items).block(surrounding_block);
        frame.render_widget(list, area);
        Ok(())
    }
}
