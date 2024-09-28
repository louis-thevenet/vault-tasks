use color_eyre::Result;
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;
use tokio::sync::mpsc::UnboundedSender;

use super::Component;

use crate::{action::Action, config::Config};

#[derive(Default)]
pub struct Tags {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    focused: bool,
}

impl Tags {
    pub fn new() -> Self {
        Self::default()
    }
}
impl Component for Tags {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.config = config;
        Ok(())
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::FocusExplorer => self.focused = false,
            Action::FocusTags => self.focused = true,
            _ => (),
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, _area: Rect) -> Result<()> {
        if !self.focused {
            return Ok(());
        }
        let vertical = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ]);
        let [_header_area, inner_area, _footer_areaa] = vertical.areas(frame.area());
        Paragraph::new("tags").render(inner_area, frame.buffer_mut());
        Ok(())
    }
}
